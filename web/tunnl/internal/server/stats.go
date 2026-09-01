package server

import (
	"encoding/json"
	"log"
	"net"
	"net/http"
	"sort"
	"sync/atomic"
	"time"
)

// Stats holds server statistics
type Stats struct {
	ActiveTunnels    int          `json:"active_tunnels"`
	TotalConnections uint64       `json:"total_connections"`
	TotalRequests    uint64       `json:"total_requests"`
	Tunnels          []TunnelInfo `json:"tunnels,omitempty"`
}

// TunnelInfo describes one active tunnel
type TunnelInfo struct {
	Subdomain string    `json:"subdomain"`
	ClientIP  string    `json:"client_ip"`
	CreatedAt time.Time `json:"created_at"`
}

// IncrementConnections increments the total connection counter
func (s *Server) IncrementConnections() {
	atomic.AddUint64(&s.totalConnections, 1)
}

// IncrementRequests increments the total request counter
func (s *Server) IncrementRequests() {
	atomic.AddUint64(&s.totalRequests, 1)
}

// GetStats returns current server statistics
func (s *Server) GetStats(includeTunnels bool) Stats {
	s.mu.RLock()
	defer s.mu.RUnlock()

	stats := Stats{
		ActiveTunnels:    len(s.tunnels),
		TotalConnections: atomic.LoadUint64(&s.totalConnections),
		TotalRequests:    atomic.LoadUint64(&s.totalRequests),
	}

	if includeTunnels {
		stats.Tunnels = make([]TunnelInfo, 0, len(s.tunnels))
		for sub, t := range s.tunnels {
			stats.Tunnels = append(stats.Tunnels, TunnelInfo{
				Subdomain: sub,
				ClientIP:  t.ClientIP,
				CreatedAt: t.CreatedAt,
			})
		}
		sort.Slice(stats.Tunnels, func(i, j int) bool {
			return stats.Tunnels[i].Subdomain < stats.Tunnels[j].Subdomain
		})
	}

	return stats
}

// StatsHandler returns an http.Handler for the stats endpoint
func (s *Server) StatsHandler() http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		// Only allow from localhost
		host, _, err := net.SplitHostPort(r.RemoteAddr)
		if err != nil {
			host = r.RemoteAddr
		}
		ip := net.ParseIP(host)
		if ip == nil || !ip.IsLoopback() {
			http.Error(w, "Forbidden", http.StatusForbidden)
			return
		}

		includeTunnels := r.URL.Query().Get("tunnels") == "true"
		stats := s.GetStats(includeTunnels)

		w.Header().Set("Content-Type", "application/json")
		if err := json.NewEncoder(w).Encode(stats); err != nil {
			log.Printf("Failed to encode stats response: %v", err)
		}
	})
}
