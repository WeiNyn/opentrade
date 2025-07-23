# OpenTrade Docker Setup

This document provides instructions for running the OpenTrade streaming klines application using Docker Compose with TimescaleDB and Grafana.

## Architecture

The Docker setup includes the following services:

- **TimescaleDB**: PostgreSQL with TimescaleDB extension for time-series data
- **Streaming Klines**: Rust application that streams live kline data from Binance
- **Backfill Klines**: Rust application for historical data backfill (optional)
- **Grafana**: Data visualization dashboard
- **Migrations**: Database schema setup service

## Prerequisites

- Docker Engine 20.10+ 
- Docker Compose 2.0+
- At least 2GB of available RAM
- Internet connection for Binance WebSocket API

## Quick Start

1. **Clone the repository and navigate to the project directory**
   ```bash
   git clone <repository-url>
   cd opentrade
   ```

2. **Start the services**
   ```bash
   docker-compose up -d
   ```

3. **Monitor the logs**
   ```bash
   docker-compose logs -f streaming-klines
   ```

4. **Access Grafana dashboard**
   - Open http://localhost:3000 in your browser
   - Login with username: `admin`, password: `admin123`
   - Navigate to the "OpenTrade - Crypto Trading Dashboard"

## Service Details

### TimescaleDB
- **Port**: 5432
- **Database**: postgres
- **Username**: postgres
- **Password**: password
- **Data Volume**: `opentrade-timescaledb-data`

### Streaming Klines Service
- Connects to Binance WebSocket API
- Streams BTCUSDT 1-minute klines
- Stores data in TimescaleDB
- Auto-restarts on failure

### Grafana
- **Port**: 3000
- **Admin User**: admin
- **Admin Password**: admin123 (configurable)
- **Data Volume**: `opentrade-grafana-data`

## Configuration

### Environment Variables

The main configuration is in `docker/.env`:

```env
# Database Configuration
DATABASE_URL=postgres://postgres:password@timescaledb:5432/postgres

# Application Configuration
RUST_LOG=info
RUST_BACKTRACE=0

# Grafana Configuration
GRAFANA_ADMIN_PASSWORD=admin123
```

### Development Overrides

For development, the `docker-compose.override.yml` file provides:
- Verbose logging (`RUST_LOG=debug`)
- Additional development tools (pgAdmin, Redis)
- Source code mounting for live development

To use development tools:
```bash
docker-compose --profile dev-tools up -d
```

## Common Operations

### View Service Status
```bash
docker-compose ps
```

### View Logs
```bash
# All services
docker-compose logs

# Specific service
docker-compose logs streaming-klines
docker-compose logs timescaledb
docker-compose logs grafana
```

### Restart Services
```bash
# Restart all services
docker-compose restart

# Restart specific service
docker-compose restart streaming-klines
```

### Run Backfill (Historical Data)
```bash
docker-compose --profile backfill up backfill-klines
```

### Access Database Directly
```bash
docker-compose exec timescaledb psql -U postgres -d postgres
```

### Clean Up
```bash
# Stop services
docker-compose down

# Stop and remove volumes (WARNING: deletes all data)
docker-compose down -v

# Remove images
docker-compose down --rmi all
```

## Database Schema

The database automatically sets up the following table:

```sql
CREATE TABLE kline_data (
    start_time TIMESTAMPTZ NOT NULL,
    end_time TIMESTAMPTZ NOT NULL,
    symbol VARCHAR(20) NOT NULL,
    interval VARCHAR(10) NOT NULL,
    first_trade_id INTEGER NOT NULL,
    last_trade_id INTEGER NOT NULL,
    open DECIMAL(20,8) NOT NULL,
    high DECIMAL(20,8) NOT NULL,
    low DECIMAL(20,8) NOT NULL,
    close DECIMAL(20,8) NOT NULL,
    volume DECIMAL(20,8) NOT NULL,
    trade_count INTEGER,
    quote_volume DECIMAL(20,8),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    update_at TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (start_time, symbol, interval)
);

-- TimescaleDB hypertable for efficient time-series operations
SELECT create_hypertable('kline_data', 'start_time', chunk_time_interval => INTERVAL '1 day');
```

## Grafana Dashboard

The included dashboard provides:
- Real-time BTCUSDT price chart
- Current price gauge
- Trading volume visualization
- Data statistics and health metrics

### Custom Queries

Example queries for additional analysis:

```sql
-- Price changes in the last hour
SELECT 
    start_time,
    close,
    LAG(close) OVER (ORDER BY start_time) as prev_close,
    (close - LAG(close) OVER (ORDER BY start_time)) / LAG(close) OVER (ORDER BY start_time) * 100 as price_change_pct
FROM kline_data 
WHERE symbol = 'BTCUSDT' 
    AND interval = '1m' 
    AND start_time >= NOW() - INTERVAL '1 hour'
ORDER BY start_time;

-- Volume-weighted average price (VWAP)
SELECT 
    start_time,
    SUM(close * volume) OVER (ORDER BY start_time ROWS BETWEEN 19 PRECEDING AND CURRENT ROW) / 
    SUM(volume) OVER (ORDER BY start_time ROWS BETWEEN 19 PRECEDING AND CURRENT ROW) as vwap_20
FROM kline_data 
WHERE symbol = 'BTCUSDT' 
    AND interval = '1m'
ORDER BY start_time DESC
LIMIT 100;
```

## Troubleshooting

### Service Won't Start
1. Check Docker daemon is running
2. Verify port availability (5432, 3000)
3. Check logs: `docker-compose logs <service-name>`

### Database Connection Issues
1. Verify TimescaleDB health: `docker-compose exec timescaledb pg_isready`
2. Check database logs: `docker-compose logs timescaledb`
3. Ensure migrations completed: `docker-compose logs migrations`

### No Data in Grafana
1. Verify streaming service is running: `docker-compose logs streaming-klines`
2. Check WebSocket connection to Binance
3. Verify database has data: `docker-compose exec timescaledb psql -U postgres -c "SELECT COUNT(*) FROM kline_data;"`

### Performance Issues
1. Increase Docker memory allocation (recommended: 4GB+)
2. Monitor resource usage: `docker stats`
3. Check TimescaleDB performance: View slow queries in logs

## Production Considerations

For production deployment:

1. **Security**:
   - Change default passwords
   - Use Docker secrets for sensitive data
   - Configure proper network isolation
   - Enable SSL/TLS connections

2. **Monitoring**:
   - Add health check endpoints
   - Implement log aggregation
   - Set up alerting for service failures

3. **Data Persistence**:
   - Configure backup strategies
   - Use external volume storage
   - Implement data retention policies

4. **Scaling**:
   - Use Docker Swarm or Kubernetes
   - Implement horizontal scaling for services
   - Configure load balancing

## Support

For issues and questions:
1. Check the application logs
2. Review this documentation
3. Consult the TimescaleDB and Grafana documentation
4. File an issue in the project repository