package server

import (
	"os"
	"path/filepath"
	"testing"

	"golang.org/x/crypto/ssh"
)

const testAuthorizedKey = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIHavg+rhFmR2p9wuWiO4VxKaIXpq1gOm17jCoZ9jMxvL haylin@haytop"

func writeKeys(t *testing.T, contents string) string {
	t.Helper()
	path := filepath.Join(t.TempDir(), "authorized_keys")
	if err := os.WriteFile(path, []byte(contents), 0600); err != nil {
		t.Fatalf("failed to write authorized keys: %v", err)
	}
	return path
}

func TestLoadAuthorizedKeys(t *testing.T) {
	keys, err := loadAuthorizedKeys(writeKeys(t, "# a comment\n\n"+testAuthorizedKey+"\n"))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(keys) != 1 {
		t.Fatalf("expected 1 key, got %d", len(keys))
	}

	parsed, _, _, _, err := ssh.ParseAuthorizedKey([]byte(testAuthorizedKey))
	if err != nil {
		t.Fatalf("failed to parse test key: %v", err)
	}
	comment, ok := keys.lookup(parsed)
	if !ok {
		t.Fatal("authorized key was not accepted")
	}
	if comment != "haylin@haytop" {
		t.Errorf("expected comment haylin@haytop, got %q", comment)
	}
}

func TestLoadAuthorizedKeysRejects(t *testing.T) {
	tests := map[string]string{
		"empty file":   "\n# nothing here\n",
		"garbage line": "ssh-ed25519 not-a-key\n",
	}
	for name, contents := range tests {
		t.Run(name, func(t *testing.T) {
			if _, err := loadAuthorizedKeys(writeKeys(t, contents)); err == nil {
				t.Error("expected an error, got nil")
			}
		})
	}
}

func TestLoadAuthorizedKeysMissingFile(t *testing.T) {
	if _, err := loadAuthorizedKeys(filepath.Join(t.TempDir(), "nope")); err == nil {
		t.Error("expected an error for a missing file, got nil")
	}
}

func TestUnknownKeyIsRejected(t *testing.T) {
	keys, err := loadAuthorizedKeys(writeKeys(t, testAuthorizedKey))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	other, _, _, _, err := ssh.ParseAuthorizedKey([]byte("ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAICKsUHsNfWi9qEivDXP146uGBnW2H1m4tOW+An0b3MkZ infra@hayl.in"))
	if err != nil {
		t.Fatalf("failed to parse key: %v", err)
	}
	if _, ok := keys.lookup(other); ok {
		t.Error("unlisted key was accepted")
	}
}
