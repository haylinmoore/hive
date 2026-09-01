package subdomain

import (
	"crypto/rand"
	"encoding/hex"
	"fmt"
)

var adjectives = []string{
	"happy", "sunny", "swift", "calm", "bold", "bright", "cool", "warm",
	"quick", "clever", "brave", "gentle", "kind", "proud", "wise", "keen",
	"fresh", "crisp", "pure", "clear", "wild", "free", "silent", "quiet",
	"golden", "silver", "coral", "amber", "jade", "ruby", "pearl", "onyx",
}

var nouns = []string{
	"tiger", "eagle", "wolf", "bear", "hawk", "fox", "deer", "owl",
	"river", "mountain", "forest", "ocean", "meadow", "valley", "canyon", "island",
	"star", "moon", "cloud", "storm", "wind", "flame", "wave", "stone",
	"maple", "cedar", "pine", "oak", "willow", "birch", "aspen", "elm",
}

// Generate creates a random memorable subdomain in the format adjective-noun-hex
func Generate() (string, error) {
	adjIdx := make([]byte, 1)
	nounIdx := make([]byte, 1)
	hexBytes := make([]byte, 4) // 4 bytes = 8 hex characters for better entropy

	if _, err := rand.Read(adjIdx); err != nil {
		return "", fmt.Errorf("failed to generate random bytes: %w", err)
	}
	if _, err := rand.Read(nounIdx); err != nil {
		return "", fmt.Errorf("failed to generate random bytes: %w", err)
	}
	if _, err := rand.Read(hexBytes); err != nil {
		return "", fmt.Errorf("failed to generate random bytes: %w", err)
	}

	adj := adjectives[int(adjIdx[0])%len(adjectives)]
	noun := nouns[int(nounIdx[0])%len(nouns)]
	hexSuffix := hex.EncodeToString(hexBytes)

	return fmt.Sprintf("%s-%s-%s", adj, noun, hexSuffix), nil
}

// MaxLength is the longest a subdomain may be, per the DNS label limit.
const MaxLength = 63

// IsValid reports whether s is usable as a single DNS label under the
// wildcard certificate: lowercase alphanumerics and hyphens, not starting or
// ending with a hyphen. Dots are rejected because *.<domain> only covers one
// level.
func IsValid(s string) bool {
	if len(s) == 0 || len(s) > MaxLength {
		return false
	}
	if s[0] == '-' || s[len(s)-1] == '-' {
		return false
	}
	for _, c := range s {
		switch {
		case c >= 'a' && c <= 'z':
		case c >= '0' && c <= '9':
		case c == '-':
		default:
			return false
		}
	}
	return true
}
