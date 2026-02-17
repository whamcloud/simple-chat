package client

import (
	"bufio"
	"context"
	"fmt"
	"io"
	"sync"

	pb "github.com/arunksaha/simple-chat/gen/chat/v1"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

type recvOnly interface {
	Recv() (*pb.ServerEvent, error)
}

// WaitForJoinAck blocks until the server acknowledges join success (Joined)
// or returns a fatal pre-join error event.
func WaitForJoinAck(stream recvOnly, out io.Writer) error {
	for {
		ev, err := stream.Recv()
		if err != nil {
			return err
		}
		PrintServerEvent(out, ev)
		if ev.GetJoined() != nil {
			return nil
		}
		if e := ev.GetError(); e != nil {
			return fmt.Errorf("join failed: %s", e.Code)
		}
	}
}

// Run starts an interactive client session over a bidi gRPC chat stream.
func Run(cfg Config, in io.Reader, out io.Writer, errW io.Writer) error {
	conn, err := grpc.NewClient(cfg.Addr(), grpc.WithTransportCredentials(insecure.NewCredentials()))
	if err != nil {
		return err
	}
	defer conn.Close()

	cli := pb.NewChatServiceClient(conn)

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	stream, err := cli.Chat(ctx)
	if err != nil {
		return err
	}

	// Protocol requires Join to be the first event on a new stream.
	if err := stream.Send(&pb.ClientEvent{Kind: &pb.ClientEvent_Join{Join: &pb.Join{UserId: cfg.UserID}}}); err != nil {
		return err
	}
	// Wait for join handshake completion before starting the background receiver.
	// This avoids a race where both handshake code and recv goroutine try to
	// consume the same initial Joined/Error server event.
	if err := WaitForJoinAck(stream, out); err != nil {
		cancel()
		return err
	}

	var wg sync.WaitGroup
	recvErrCh := make(chan error, 1)
	wg.Add(1)
	go func() {
		defer wg.Done()
		// Keep receiving server events while stdin loop handles user commands.
		for {
			ev, err := stream.Recv()
			if err != nil {
				recvErrCh <- err
				return
			}
			PrintServerEvent(out, ev)
		}
	}()

	scanner := bufio.NewScanner(in)
	fmt.Fprintln(out, "commands: send <MSG> | leave")
	for scanner.Scan() {
		cmd := ParseCommand(scanner.Text())
		switch cmd.Kind {
		case CmdSend:
			if cmd.Arg == "" {
				fmt.Fprintln(errW, "send requires a message")
				continue
			}
			if err := stream.Send(&pb.ClientEvent{Kind: &pb.ClientEvent_Send{Send: &pb.Send{Text: cmd.Arg}}}); err != nil {
				cancel()
				wg.Wait()
				return err
			}
		case CmdLeave:
			_ = stream.Send(&pb.ClientEvent{Kind: &pb.ClientEvent_Leave{Leave: &pb.Leave{}}})
			cancel()
			wg.Wait()
			return nil
		default:
			fmt.Fprintln(errW, "unknown command. use: send <MSG> | leave")
		}

		// Surface stream-side failures quickly without blocking command input.
		select {
		case err := <-recvErrCh:
			cancel()
			wg.Wait()
			return err
		default:
		}
	}
	if err := scanner.Err(); err != nil {
		cancel()
		wg.Wait()
		return err
	}
	cancel()
	wg.Wait()
	return nil
}
