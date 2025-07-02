-- Create uniq constaint
ALTER TABLE kline_data ADD CONSTRAINT unique_kline_data UNIQUE (start_time, symbol, interval);

-- Create primary key
ALTER TABLE kline_data ADD PRIMARY KEY (start_time, symbol, interval);