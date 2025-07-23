-- TimescaleDB initialization script
-- This script sets up the TimescaleDB extension and configures the database for opentrade

-- Enable TimescaleDB extension
CREATE EXTENSION IF NOT EXISTS timescaledb;

-- Create the opentrade database user (if not exists)
DO
$do$
BEGIN
   IF NOT EXISTS (
      SELECT FROM pg_catalog.pg_roles
      WHERE  rolname = 'opentrade') THEN

      CREATE ROLE opentrade LOGIN PASSWORD 'opentrade_password';
   END IF;
END
$do$;

-- Grant privileges to opentrade user
GRANT ALL PRIVILEGES ON DATABASE postgres TO opentrade;
GRANT ALL ON SCHEMA public TO opentrade;

-- Set default privileges for future tables
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON TABLES TO opentrade;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON SEQUENCES TO opentrade;

-- Configure TimescaleDB settings for optimal performance
-- Set chunk time interval for better partitioning
SET timescaledb.max_background_workers = 8;

-- Create a dedicated schema for monitoring (optional)
CREATE SCHEMA IF NOT EXISTS monitoring;
GRANT ALL ON SCHEMA monitoring TO opentrade;

-- Log initialization completion
SELECT 'TimescaleDB initialization completed successfully' AS status;