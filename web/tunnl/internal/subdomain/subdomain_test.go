package subdomain

import (
	"strings"
	"testing"
)

func TestGenerate(t *testing.T) {
	t.Run("format", func(t *testing.T) {
		sub, err := Generate()
		if err != nil {
			t.Fatalf("Generate() error: %v", err)
		}
		if !IsValid(sub) {
			t.Errorf("Generate() produced invalid subdomain: %q", sub)
		}
	})

	t.Run("uniqueness", func(t *testing.T) {
		seen := make(map[string]struct{})
		for i := 0; i < 100; i++ {
			sub, err := Generate()
			if err != nil {
				t.Fatalf("Generate() error on iteration %d: %v", i, err)
			}
			if _, ok := seen[sub]; ok {
				t.Fatalf("Generate() produced duplicate subdomain %q on iteration %d", sub, i)
			}
			seen[sub] = struct{}{}
		}
	})
}

func TestIsValid(t *testing.T) {
	tests := []struct {
		input string
		want  bool
	}{
		{"myapp", true},
		{"my-app", true},
		{"a", true},
		{"happy-tiger-a1b2c3d4", true},
		{"api2", true},
		{strings.Repeat("a", MaxLength), true},
		{"", false},
		{strings.Repeat("a", MaxLength+1), false},
		{"-leading", false},
		{"trailing-", false},
		{"MyApp", false},
		{"my_app", false},
		{"my.app", false},
		{"my app", false},
		{"café", false},
	}

	for _, tt := range tests {
		t.Run(tt.input, func(t *testing.T) {
			if got := IsValid(tt.input); got != tt.want {
				t.Errorf("IsValid(%q) = %v, want %v", tt.input, got, tt.want)
			}
		})
	}
}
