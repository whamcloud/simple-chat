package client

import "testing"

func TestParseCommand(t *testing.T) {
	tests := []struct {
		name string
		line string
		want Command
	}{
		{
			name: "empty line",
			line: "",
			want: Command{Kind: CmdUnknown},
		},
		{
			name: "whitespace line",
			line: "   \t  ",
			want: Command{Kind: CmdUnknown},
		},
		{
			name: "leave command",
			line: "leave",
			want: Command{Kind: CmdLeave},
		},
		{
			name: "send command",
			line: "send hello",
			want: Command{Kind: CmdSend, Arg: "hello"},
		},
		{
			name: "send command with extra spaces",
			line: "  send   hello world  ",
			want: Command{Kind: CmdSend, Arg: "hello world"},
		},
		{
			name: "unknown command",
			line: "join",
			want: Command{Kind: CmdUnknown},
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			got := ParseCommand(tc.line)
			if got.Kind != tc.want.Kind || got.Arg != tc.want.Arg {
				t.Fatalf("ParseCommand(%q) = %#v, want %#v", tc.line, got, tc.want)
			}
		})
	}
}
