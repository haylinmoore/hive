package server

import (
	"fmt"
	"os"
	"strings"

	"golang.org/x/crypto/ssh"
)

// authorizedKeys holds the set of public keys permitted to open tunnels,
// keyed by the wire encoding of the key and valued by its comment.
type authorizedKeys map[string]string

// loadAuthorizedKeys parses an OpenSSH authorized_keys file. An empty file is
// an error: an instance nobody can connect to is never what was intended.
func loadAuthorizedKeys(path string) (authorizedKeys, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("failed to read authorized keys: %w", err)
	}

	keys := make(authorizedKeys)
	for n, line := range strings.Split(string(data), "\n") {
		line = strings.TrimSpace(line)
		if line == "" || strings.HasPrefix(line, "#") {
			continue
		}
		key, comment, _, _, err := ssh.ParseAuthorizedKey([]byte(line))
		if err != nil {
			return nil, fmt.Errorf("%s line %d: %w", path, n+1, err)
		}
		keys[string(key.Marshal())] = comment
	}

	if len(keys) == 0 {
		return nil, fmt.Errorf("no keys in %s", path)
	}
	return keys, nil
}

func (a authorizedKeys) lookup(key ssh.PublicKey) (string, bool) {
	comment, ok := a[string(key.Marshal())]
	return comment, ok
}
