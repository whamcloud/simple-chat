package client

import (
	"fmt"
	"io"

	pb "github.com/arunksaha/simple-chat/gen/chat/v1"
)

// PrintServerEvent maps wire events into simple terminal output.
func PrintServerEvent(w io.Writer, ev *pb.ServerEvent) {
	if m := ev.GetMessage(); m != nil {
		fmt.Fprintf(w, "[%s] %s\n", m.FromUserId, m.Text)
		return
	}
	if j := ev.GetJoined(); j != nil {
		fmt.Fprintf(w, "joined as %s\n", j.UserId)
		return
	}
	if e := ev.GetError(); e != nil {
		fmt.Fprintf(w, "ERROR %s: %s\n", e.Code, e.Detail)
		return
	}
	fmt.Fprintln(w, "unknown server event")
}
