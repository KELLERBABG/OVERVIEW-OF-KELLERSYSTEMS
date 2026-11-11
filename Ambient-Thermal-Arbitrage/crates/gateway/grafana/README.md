# ATA Gateway — Grafana Dashboard

Ready-made dashboard for the ATA gateway's Prometheus metrics (`GET /metrics`).

## Provisioning

1. Add a Prometheus datasource pointing at the gateway (see `DEPLOYMENT.md` §8):

```yaml
# provisioning/datasources/prometheus.yml
apiVersion: 1
datasources:
  - name: Prometheus
    type: prometheus
    access: proxy
    url: http://localhost:9090
    isDefault: true
```

2. Import `ata-dashboard.json` (Grafana → Dashboards → Import, or auto-provision):

```yaml
# provisioning/dashboards/ata.yml
apiVersion: 1
providers:
  - name: ata
    orgId: 1
    folder: ATA
    type: file
    options:
      path: /etc/grafana/dashboards/ata
      foldersFromFilesStructure: false
```

Copy `ata-dashboard.json` to `/etc/grafana/dashboards/ata/`.

## Panels

| Panel | Query |
|---|---|
| Internal temperature (°C) | `ata_internal_temp_c / 100` (metric is °C × 100) |
| Electricity price (€/MWh) | `ata_price_eur_per_mwh / 100` (metric is €/MWh × 100) |
| Time-to-failure (s) | `ata_ttf_seconds` (red below 900 s = Emergency zone) |
| Compressor state | `ata_compressor_on` |
| Control mode activations | `rate(ata_control_mode_total[5m])` by mode |
| Alerts by severity | `rate(ata_alerts_total[5m])` by severity |
| Control loop p95 duration | `histogram_quantile(0.95, rate(ata_loop_duration_seconds_bucket[5m]))` |
| HTTP requests by route | `rate(ata_http_requests_total[5m])` by path/status |

## Alerting

For cold-chain-critical alerts prefer the Prometheus rules in `DEPLOYMENT.md` §8
(e.g. `ata_control_mode_total{mode="Emergency"} > 0`) or Grafana alert rules
on the TTF panel — threshold 900 s matches `ttf_critical_sec`.
