# Home Router Exporter

Home Router Exporter is a Prometheus exporter designed for home routers.

It is secure, fast, and has a small memory footprint.  It has no external
dependencies and should run on any minimalist Linux distro.

It exports and only exports metrics related to

 - system health: cpu, memory, storage, and thermal
 - network health: link state, link stats, and route info
 - service health: DHCP and DNS

There is also a [Grafana
dashboard](https://grafana.com/grafana/dashboards/23067-home-router/) for
visualization.
