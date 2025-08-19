# NeoRust Monitoring Deployment Guide

## Production Deployment

### Prerequisites

- Kubernetes cluster (1.20+)
- Helm 3.x installed
- kubectl configured
- Storage class available for persistent volumes

### 1. Deploy Prometheus Stack

```bash
# Add Prometheus community Helm repository
helm repo add prometheus-community https://prometheus-community.github.io/helm-charts
helm repo update

# Create monitoring namespace
kubectl create namespace monitoring

# Install Prometheus stack
helm install prometheus prometheus-community/kube-prometheus-stack \
  --namespace monitoring \
  --set prometheus.prometheusSpec.serviceMonitorSelectorNilUsesHelmValues=false \
  --set grafana.adminPassword=neorust-admin \
  --values prometheus-values.yaml
```

### 2. Deploy Jaeger

```bash
# Add Jaeger Helm repository
helm repo add jaegertracing https://jaegertracing.github.io/helm-charts
helm repo update

# Install Jaeger
helm install jaeger jaegertracing/jaeger \
  --namespace monitoring \
  --set provisionDataStore.cassandra=false \
  --set storage.type=elasticsearch \
  --set elasticsearch.nodeCount=3 \
  --set elasticsearch.resources.requests.memory=2Gi \
  --set elasticsearch.resources.requests.cpu=1
```

### 3. Deploy Loki Stack

```bash
# Add Grafana Helm repository
helm repo add grafana https://grafana.github.io/helm-charts
helm repo update

# Install Loki
helm install loki grafana/loki-stack \
  --namespace monitoring \
  --set loki.persistence.enabled=true \
  --set loki.persistence.size=10Gi \
  --set promtail.enabled=true
```

### 4. Deploy OpenTelemetry Collector

```yaml
# otel-collector-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: otel-collector
  namespace: monitoring
spec:
  replicas: 2
  selector:
    matchLabels:
      app: otel-collector
  template:
    metadata:
      labels:
        app: otel-collector
    spec:
      containers:
      - name: otel-collector
        image: otel/opentelemetry-collector-contrib:latest
        ports:
        - containerPort: 4317  # OTLP gRPC
        - containerPort: 4318  # OTLP HTTP
        - containerPort: 8888  # Prometheus metrics
        volumeMounts:
        - name: config
          mountPath: /etc/otel-collector
        command:
        - "/otelcol-contrib"
        - "--config=/etc/otel-collector/config.yaml"
        resources:
          limits:
            memory: 1Gi
            cpu: 1000m
          requests:
            memory: 512Mi
            cpu: 500m
      volumes:
      - name: config
        configMap:
          name: otel-collector-config
---
apiVersion: v1
kind: Service
metadata:
  name: otel-collector
  namespace: monitoring
spec:
  selector:
    app: otel-collector
  ports:
  - name: otlp-grpc
    port: 4317
    targetPort: 4317
  - name: otlp-http
    port: 4318
    targetPort: 4318
  - name: metrics
    port: 8888
    targetPort: 8888
```

Apply the configuration:

```bash
kubectl apply -f otel-collector-deployment.yaml
```

### 5. Configure NeoRust Application

```yaml
# neorust-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: neorust-app
  namespace: default
spec:
  replicas: 3
  selector:
    matchLabels:
      app: neorust
  template:
    metadata:
      labels:
        app: neorust
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "9090"
        prometheus.io/path: "/metrics"
    spec:
      containers:
      - name: neorust
        image: your-registry/neorust:latest
        env:
        - name: NEO_METRICS_ENABLED
          value: "true"
        - name: NEO_METRICS_PORT
          value: "9090"
        - name: NEO_TRACING_ENABLED
          value: "true"
        - name: NEO_TRACING_ENDPOINT
          value: "http://otel-collector.monitoring:4317"
        - name: NEO_LOG_LEVEL
          value: "info"
        - name: NEO_HEALTH_CHECK_ENABLED
          value: "true"
        - name: NEO_HEALTH_CHECK_PORT
          value: "8080"
        ports:
        - name: metrics
          containerPort: 9090
        - name: health
          containerPort: 8080
        livenessProbe:
          httpGet:
            path: /health/liveness
            port: health
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /health/readiness
            port: health
          initialDelaySeconds: 10
          periodSeconds: 5
        resources:
          limits:
            memory: 2Gi
            cpu: 2000m
          requests:
            memory: 1Gi
            cpu: 1000m
```

### 6. Create ServiceMonitor

```yaml
# servicemonitor.yaml
apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: neorust
  namespace: default
  labels:
    app: neorust
spec:
  selector:
    matchLabels:
      app: neorust
  endpoints:
  - port: metrics
    interval: 30s
    path: /metrics
    scrapeTimeout: 10s
```

### 7. Configure Horizontal Pod Autoscaler

```yaml
# hpa.yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: neorust-hpa
  namespace: default
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: neorust-app
  minReplicas: 3
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
  - type: Pods
    pods:
      metric:
        name: neo_transactions_per_second
      target:
        type: AverageValue
        averageValue: "100"
```

## Cloud Provider Specific Setup

### AWS

```bash
# Install AWS CloudWatch Container Insights
kubectl apply -f https://raw.githubusercontent.com/aws-samples/amazon-cloudwatch-container-insights/latest/k8s-deployment-manifest-templates/deployment-mode/daemonset/container-insights-monitoring/quickstart/cwagent-fluentd-quickstart.yaml

# Configure ALB Ingress for Grafana
kubectl apply -f - <<EOF
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: grafana-ingress
  namespace: monitoring
  annotations:
    kubernetes.io/ingress.class: alb
    alb.ingress.kubernetes.io/scheme: internet-facing
    alb.ingress.kubernetes.io/certificate-arn: arn:aws:acm:region:account:certificate/id
spec:
  rules:
  - host: grafana.neorust.example.com
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: prometheus-grafana
            port:
              number: 80
EOF
```

### GCP

```bash
# Enable Google Cloud Monitoring
gcloud services enable monitoring.googleapis.com

# Install GKE monitoring
kubectl apply -f https://raw.githubusercontent.com/GoogleCloudPlatform/k8s-stackdriver/master/prometheus-service-monitoring/prometheus-service-monitoring.yaml
```

### Azure

```bash
# Enable Azure Monitor for containers
az aks enable-addons \
  --resource-group myResourceGroup \
  --name myAKSCluster \
  --addons monitoring
```

## Security Configuration

### 1. Enable TLS

```yaml
# tls-config.yaml
apiVersion: v1
kind: Secret
metadata:
  name: monitoring-tls
  namespace: monitoring
type: kubernetes.io/tls
data:
  tls.crt: # base64 encoded certificate
  tls.key: # base64 encoded private key
```

### 2. Configure RBAC

```yaml
# rbac.yaml
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata:
  name: monitoring-reader
rules:
- apiGroups: [""]
  resources: ["pods", "services", "endpoints"]
  verbs: ["get", "list", "watch"]
- apiGroups: ["apps"]
  resources: ["deployments", "replicasets"]
  verbs: ["get", "list", "watch"]
---
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRoleBinding
metadata:
  name: monitoring-reader-binding
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: ClusterRole
  name: monitoring-reader
subjects:
- kind: ServiceAccount
  name: prometheus
  namespace: monitoring
```

### 3. Network Policies

```yaml
# network-policy.yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: monitoring-network-policy
  namespace: monitoring
spec:
  podSelector:
    matchLabels:
      app: prometheus
  policyTypes:
  - Ingress
  - Egress
  ingress:
  - from:
    - namespaceSelector:
        matchLabels:
          name: monitoring
    ports:
    - protocol: TCP
      port: 9090
  egress:
  - to:
    - podSelector: {}
    ports:
    - protocol: TCP
      port: 9090
```

## Backup and Recovery

### Prometheus Data Backup

```bash
# Create backup
kubectl exec -n monitoring prometheus-0 -- tar czf /tmp/prometheus-backup.tar.gz /prometheus

# Copy backup to local
kubectl cp monitoring/prometheus-0:/tmp/prometheus-backup.tar.gz ./prometheus-backup.tar.gz

# Restore backup
kubectl cp ./prometheus-backup.tar.gz monitoring/prometheus-0:/tmp/
kubectl exec -n monitoring prometheus-0 -- tar xzf /tmp/prometheus-backup.tar.gz -C /
```

### Grafana Dashboard Backup

```bash
# Export all dashboards
for dashboard in $(curl -s http://admin:password@grafana.monitoring.svc.cluster.local:3000/api/search | jq -r '.[] | .uid'); do
  curl -s http://admin:password@grafana.monitoring.svc.cluster.local:3000/api/dashboards/uid/$dashboard \
    > dashboard-$dashboard.json
done

# Import dashboards
for file in dashboard-*.json; do
  curl -X POST http://admin:password@grafana.monitoring.svc.cluster.local:3000/api/dashboards/db \
    -H "Content-Type: application/json" \
    -d @$file
done
```

## Monitoring Best Practices

1. **Resource Allocation**
   - Allocate sufficient resources for monitoring components
   - Use persistent volumes for data retention
   - Implement resource quotas

2. **Data Retention**
   - Configure appropriate retention periods
   - Implement data archival strategies
   - Use remote storage for long-term retention

3. **High Availability**
   - Deploy multiple replicas of critical components
   - Use anti-affinity rules for pod distribution
   - Implement cross-region replication

4. **Performance Optimization**
   - Tune scrape intervals based on requirements
   - Use recording rules for frequently queried metrics
   - Implement metric cardinality limits

5. **Alert Fatigue Prevention**
   - Set appropriate alert thresholds
   - Implement alert grouping and inhibition
   - Use escalation policies

## Troubleshooting

### Common Issues

1. **High Memory Usage**
```bash
# Check memory usage
kubectl top pods -n monitoring

# Increase memory limits
kubectl set resources deployment/prometheus -n monitoring --limits=memory=4Gi
```

2. **Metrics Not Appearing**
```bash
# Check service discovery
kubectl exec -n monitoring prometheus-0 -- curl localhost:9090/api/v1/targets

# Verify ServiceMonitor
kubectl get servicemonitor -n default -o yaml
```

3. **Trace Export Failures**
```bash
# Check OTLP collector logs
kubectl logs -n monitoring deployment/otel-collector

# Verify network connectivity
kubectl exec -n default neorust-pod -- nc -zv otel-collector.monitoring 4317
```