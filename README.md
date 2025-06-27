# OpenMeteo MCP Server

A Model Context Protocol (MCP) server that provides access to weather data and forecasts through the OpenMeteo API. This server enables AI assistants to retrieve current weather conditions, forecasts, historical weather data, and search for locations worldwide.

## Features

- **Current Weather**: Get real-time weather conditions including temperature, humidity, precipitation, wind, and atmospheric pressure
- **Weather Forecasts**: Retrieve detailed weather forecasts for up to 16 days
- **Historical Weather**: Access historical weather data for analysis and comparison
- **Location Search**: Find coordinates and details for cities and locations worldwide
- **Free API**: Uses the free OpenMeteo API with no API key required
- **Comprehensive Data**: Includes temperature, precipitation, wind, pressure, cloud cover, and weather descriptions
- **Smart Formatting**: Human-readable weather reports with emojis and clear organization

## Installation

### Option 1: Download Pre-built Binary (Recommended)

Download the latest release from GitHub:

1. **Visit the Releases Page**: Go to [https://github.com/gbrigandi/mcp-server-openmeteo/releases](https://github.com/gbrigandi/mcp-server-openmeteo/releases)

2. **Choose Your Version**: Click on the latest release (or the specific version you want)

3. **Download for Your Platform**: In the "Assets" section, download the appropriate binary for your system:
   - **macOS (Apple Silicon)**: `mcp-server-openmeteo-aarch64-apple-darwin`
   - **macOS (Intel)**: `mcp-server-openmeteo-x86_64-apple-darwin`
   - **Linux (64-bit)**: `mcp-server-openmeteo-x86_64-unknown-linux-gnu`
   - **Windows (64-bit)**: `mcp-server-openmeteo-x86_64-pc-windows-msvc.exe`

4. **Make it Executable** (macOS/Linux only):
   ```bash
   chmod +x mcp-server-openmeteo
   ```

### Option 2: Building from Source

If you prefer to build from source or need the latest development version:

#### Prerequisites

- Rust 1.87 or newer
- Cargo package manager

#### Build Steps

```bash
git clone https://github.com/gbrigandi/mcp-server-openmeteo
cd mcp-server-openmeteo
cargo build --release
```

The compiled binary will be available at `target/release/mcp-server-openmeteo`.

## Usage

### Basic Usage

Run the server with default settings:

```bash
./target/release/mcp-server-openmeteo
```

### Environment Variables

You can control the logging level using the `RUST_LOG` environment variable:

```bash
# Enable debug logging
RUST_LOG=debug ./target/release/mcp-server-openmeteo

# Enable info logging (default)
RUST_LOG=info ./target/release/mcp-server-openmeteo

# Disable most logging
RUST_LOG=warn ./target/release/mcp-server-openmeteo
```

### MCP Client Configuration

#### Claude Desktop

Add this server to your Claude Desktop configuration file:

**macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
**Windows**: `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "openmeteo": {
      "command": "/path/to/mcp-server-openmeteo"
    }
  }
}
```

After adding the configuration:
1. Save the file
2. Restart Claude Desktop
3. The OpenMeteo weather tools will be available in your conversations

## Available Tools

### 1. get_current_weather

Get current weather conditions for a specific location. Returns real-time weather data including temperature, humidity, precipitation, wind, and atmospheric conditions.

**Parameters:**
- `latitude` (required): Latitude coordinate (-90 to 90)
- `longitude` (required): Longitude coordinate (-180 to 180)

**Returns:**
- Current temperature and "feels like" temperature
- Relative humidity percentage
- Precipitation amount
- Wind speed, direction, and gusts
- Atmospheric pressure (mean sea level)
- Cloud cover percentage
- Weather condition description
- Day/night indicator

**Example:**
```json
{
  "latitude": 40.7128,
  "longitude": -74.0060
}
```

### 2. get_weather_forecast

Get weather forecast for a specific location. Returns detailed forecast data for up to 16 days including daily temperature, precipitation, wind, and weather conditions.

**Parameters:**
- `latitude` (required): Latitude coordinate (-90 to 90)
- `longitude` (required): Longitude coordinate (-180 to 180)
- `days` (optional): Number of forecast days (1-16, default: 7)

**Returns:**
- Daily high and low temperatures
- Weather condition descriptions
- Precipitation amounts
- Maximum wind speeds
- Sunrise and sunset times
- UV index and daylight duration

**Example:**
```json
{
  "latitude": 40.7128,
  "longitude": -74.0060,
  "days": 5
}
```

### 3. get_historical_weather

Get historical weather data for a specific location and date range. Returns daily weather statistics including temperature, precipitation, and other meteorological data for analysis.

**Parameters:**
- `latitude` (required): Latitude coordinate (-90 to 90)
- `longitude` (required): Longitude coordinate (-180 to 180)
- `start_date` (required): Start date in YYYY-MM-DD format
- `end_date` (required): End date in YYYY-MM-DD format

**Returns:**
- Daily temperature statistics (min, max, mean)
- Precipitation totals and averages
- Wind speed and direction data
- Summary statistics for the entire period
- Sample daily data for the first 5 days

**Example:**
```json
{
  "latitude": 40.7128,
  "longitude": -74.0060,
  "start_date": "2024-01-01",
  "end_date": "2024-01-31"
}
```

### 4. search_locations

Search for locations by name to get their coordinates and details. Use format "city, country" where country is optional (e.g., "Paris, France" or just "Tokyo"). Returns a list of matching locations with coordinates and other geographic information.

**Parameters:**
- `query` (required): Location search query in format "city, country" (country is optional)
  - Examples: "Paris, France", "Tokyo", "New York, USA", "London"
- `limit` (optional): Maximum number of results (1-100, default: 10)

**Returns:**
- City/location name and country
- Administrative region (state/province)
- Precise coordinates (latitude/longitude)
- Timezone information
- Population data (when available)

**Example:**
```json
{
  "query": "Paris, France",
  "limit": 5
}
```
## Data Source

All weather data is provided by [OpenMeteo](https://open-meteo.com/), a free weather API that offers:

- **High-Quality Data**: Based on multiple weather models and observations
- **Global Coverage**: Worldwide weather data availability
- **No API Key Required**: Free access without registration
- **High Availability**: Reliable service with good uptime
- **Open Source**: Based on open-source weather models
- **Real-Time Updates**: Current conditions updated regularly
- **Historical Archive**: Access to historical weather data

## Performance

- **Request Timeout**: 30-second timeout for all API requests
- **Efficient Requests**: Optimized API calls with only necessary parameters
- **Error Recovery**: Graceful handling of temporary API failures
- **Coordinate Validation**: Input validation to prevent invalid API requests

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. Areas for contribution:

- Additional weather parameters
- Enhanced formatting options
- Performance optimizations
- Additional validation
- Documentation improvements

## Support

For issues and questions:

1. Check the existing issues in the repository
2. Create a new issue with detailed information about the problem
3. Include relevant logs and configuration details
4. Provide example coordinates and parameters that cause issues

## Changelog

### Version 0.1.0
- Initial release with full MCP server functionality
- Current weather conditions with comprehensive data
- Weather forecasts for up to 16 days
- Historical weather data with statistical summaries
- Location search with geocoding
