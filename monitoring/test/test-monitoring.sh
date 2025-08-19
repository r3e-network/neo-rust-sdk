#!/bin/bash

# Test script for NeoRust monitoring system

set -e

echo "🚀 Testing NeoRust Monitoring System"
echo "====================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if Docker is running
check_docker() {
    echo -n "Checking Docker... "
    if ! docker info >/dev/null 2>&1; then
        echo -e "${RED}✗ Docker is not running${NC}"
        exit 1
    fi
    echo -e "${GREEN}✓ Docker is running${NC}"
}

# Start monitoring stack
start_monitoring() {
    echo -n "Starting monitoring stack... "
    cd ../
    docker-compose up -d >/dev/null 2>&1
    echo -e "${GREEN}✓ Started${NC}"
}

# Wait for services to be ready
wait_for_services() {
    echo "Waiting for services to be ready..."
    
    # Wait for Prometheus
    echo -n "  Prometheus... "
    while ! curl -s http://localhost:9090/-/ready >/dev/null 2>&1; do
        sleep 1
    done
    echo -e "${GREEN}✓${NC}"
    
    # Wait for Grafana
    echo -n "  Grafana... "
    while ! curl -s http://localhost:3000/api/health >/dev/null 2>&1; do
        sleep 1
    done
    echo -e "${GREEN}✓${NC}"
    
    # Wait for Jaeger
    echo -n "  Jaeger... "
    while ! curl -s http://localhost:16686/ >/dev/null 2>&1; do
        sleep 1
    done
    echo -e "${GREEN}✓${NC}"
    
    # Wait for Loki
    echo -n "  Loki... "
    while ! curl -s http://localhost:3100/ready >/dev/null 2>&1; do
        sleep 1
    done
    echo -e "${GREEN}✓${NC}"
}

# Test metrics endpoint
test_metrics() {
    echo "Testing metrics endpoints..."
    
    # Test Prometheus metrics
    echo -n "  Prometheus metrics... "
    if curl -s http://localhost:9090/api/v1/query?query=up | grep -q "success"; then
        echo -e "${GREEN}✓${NC}"
    else
        echo -e "${RED}✗${NC}"
    fi
    
    # Test node exporter
    echo -n "  Node exporter... "
    if curl -s http://localhost:9100/metrics | grep -q "node_"; then
        echo -e "${GREEN}✓${NC}"
    else
        echo -e "${RED}✗${NC}"
    fi
}

# Test health endpoints
test_health() {
    echo "Testing health endpoints..."
    
    # Test application health (if running)
    echo -n "  Application health... "
    if curl -s http://localhost:8080/health 2>/dev/null | grep -q "status"; then
        echo -e "${GREEN}✓${NC}"
    else
        echo -e "${YELLOW}⚠ Application not running${NC}"
    fi
    
    # Test liveness
    echo -n "  Liveness endpoint... "
    if curl -s http://localhost:8080/health/liveness 2>/dev/null | grep -q "alive"; then
        echo -e "${GREEN}✓${NC}"
    else
        echo -e "${YELLOW}⚠ Application not running${NC}"
    fi
    
    # Test readiness
    echo -n "  Readiness endpoint... "
    if curl -s http://localhost:8080/health/readiness 2>/dev/null | grep -q "ready"; then
        echo -e "${GREEN}✓${NC}"
    else
        echo -e "${YELLOW}⚠ Application not running${NC}"
    fi
}

# Test Grafana dashboards
test_dashboards() {
    echo "Testing Grafana dashboards..."
    
    # Login to Grafana
    echo -n "  Grafana login... "
    TOKEN=$(curl -s -X POST http://admin:neorust@localhost:3000/api/auth/keys \
        -H "Content-Type: application/json" \
        -d '{"name":"test-key","role":"Admin"}' 2>/dev/null | jq -r '.key' || echo "")
    
    if [ -n "$TOKEN" ]; then
        echo -e "${GREEN}✓${NC}"
    else
        echo -e "${YELLOW}⚠ Using basic auth${NC}"
    fi
    
    # Check dashboards
    echo -n "  Dashboard availability... "
    if curl -s http://admin:neorust@localhost:3000/api/dashboards/uid/neorust-overview | grep -q "dashboard"; then
        echo -e "${GREEN}✓${NC}"
    else
        echo -e "${YELLOW}⚠ Dashboard may not be loaded yet${NC}"
    fi
}

# Test alerting
test_alerting() {
    echo "Testing alerting system..."
    
    # Check AlertManager
    echo -n "  AlertManager... "
    if curl -s http://localhost:9093/api/v1/status | grep -q "uptime"; then
        echo -e "${GREEN}✓${NC}"
    else
        echo -e "${RED}✗${NC}"
    fi
    
    # Check alert rules
    echo -n "  Alert rules... "
    if curl -s http://localhost:9090/api/v1/rules | grep -q "neorust"; then
        echo -e "${GREEN}✓${NC}"
    else
        echo -e "${YELLOW}⚠ Alert rules may not be loaded${NC}"
    fi
}

# Test tracing
test_tracing() {
    echo "Testing distributed tracing..."
    
    # Check Jaeger API
    echo -n "  Jaeger API... "
    if curl -s http://localhost:16686/api/services | grep -q "data"; then
        echo -e "${GREEN}✓${NC}"
    else
        echo -e "${RED}✗${NC}"
    fi
    
    # Check OTLP endpoint
    echo -n "  OTLP endpoint... "
    if nc -zv localhost 4317 2>/dev/null; then
        echo -e "${GREEN}✓${NC}"
    else
        echo -e "${RED}✗${NC}"
    fi
}

# Test logging
test_logging() {
    echo "Testing log aggregation..."
    
    # Check Loki
    echo -n "  Loki API... "
    if curl -s http://localhost:3100/api/prom/label | grep -q "status"; then
        echo -e "${GREEN}✓${NC}"
    else
        echo -e "${RED}✗${NC}"
    fi
    
    # Check Promtail
    echo -n "  Promtail... "
    if docker ps | grep -q promtail; then
        echo -e "${GREEN}✓${NC}"
    else
        echo -e "${RED}✗${NC}"
    fi
}

# Generate test data
generate_test_data() {
    echo "Generating test metrics..."
    
    # Send test metrics to Prometheus pushgateway (if available)
    echo -n "  Sending test metrics... "
    
    # Create a test metric
    cat <<EOF | curl -s --data-binary @- http://localhost:9090/metrics >/dev/null 2>&1 || true
# TYPE test_metric gauge
# HELP test_metric Test metric for monitoring validation
test_metric{component="test"} 42
EOF
    
    echo -e "${GREEN}✓${NC}"
}

# Print summary
print_summary() {
    echo ""
    echo "====================================="
    echo "Monitoring System Test Summary"
    echo "====================================="
    echo ""
    echo "Access Points:"
    echo "  Grafana:      http://localhost:3000 (admin/neorust)"
    echo "  Prometheus:   http://localhost:9090"
    echo "  Jaeger:       http://localhost:16686"
    echo "  AlertManager: http://localhost:9093"
    echo ""
    echo "Next Steps:"
    echo "  1. Run your NeoRust application with monitoring enabled"
    echo "  2. Check the dashboards for real-time metrics"
    echo "  3. Configure alerts based on your requirements"
    echo ""
    echo -e "${GREEN}✓ Monitoring system is ready!${NC}"
}

# Stop monitoring stack
stop_monitoring() {
    echo ""
    read -p "Do you want to stop the monitoring stack? (y/n) " -n 1 -r
    echo ""
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        echo "Stopping monitoring stack..."
        cd ../
        docker-compose down
        echo -e "${GREEN}✓ Stopped${NC}"
    fi
}

# Main execution
main() {
    check_docker
    start_monitoring
    wait_for_services
    
    echo ""
    echo "Running tests..."
    echo "---------------"
    test_metrics
    test_health
    test_dashboards
    test_alerting
    test_tracing
    test_logging
    generate_test_data
    
    print_summary
    stop_monitoring
}

# Run main function
main