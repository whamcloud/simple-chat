package client

import "strings"

// CommandKind identifies supported CLI commands.
type CommandKind int

const (
	CmdUnknown CommandKind = iota
	CmdSend
	CmdLeave
)

// Command is the parsed representation of a single input line.
type Command struct {
	Kind CommandKind
	Arg  string
}

// ParseCommand keeps parsing intentionally strict so client UX stays predictable.
func ParseCommand(line string) Command {
	line = strings.TrimSpace(line)
	if line == "" {
		return Command{Kind: CmdUnknown}
	}
	if line == "leave" {
		return Command{Kind: CmdLeave}
	}
	if strings.HasPrefix(line, "send ") {
		return Command{Kind: CmdSend, Arg: strings.TrimSpace(strings.TrimPrefix(line, "send "))}
	}
	return Command{Kind: CmdUnknown}
}
