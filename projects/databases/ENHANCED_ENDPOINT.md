# Enhanced GitHub Star Chart Endpoint

## Overview

The enhanced `/github/repo_stars/read_daily_graph` endpoint now supports:

1. **Multiple repositories** - Compare multiple repositories on the same chart
2. **Smart relative time axis** - Automatically chooses clean round number intervals (7d, 1m, 3m, 1y, 2y) based on data duration
3. **Multiple metric types** - Display position (cumulative), speed (daily), and acceleration (change in daily)
4. **Configurable charts** - Customize chart appearance and behavior

## Architecture

### Core Components

#### 1. Data Processing Utils (`src/utils/data_processing.rs`)
- **MetricType enum**: Defines position, speed, and acceleration metrics
- **ProcessedMultiRepoData**: Normalized data structure for multiple repositories
- **TimeAxis enum**: Handles both absolute dates and relative time periods
- **TimeScale system**: Automatically determines appropriate time intervals for clean charts
- **Mathematical transformations**: Converts raw star data into different metric types

#### 2. Chart Generation Utils (`src/utils/chart.rs`)
- **ChartConfig**: Configurable chart appearance (colors, size, legend)
- **Multi-repository support**: Overlays multiple data series with different colors
- **Smart time axis**: Automatically chooses clean intervals based on data duration
- **Smart formatting**: Automatic value formatting (K, M suffixes) and time labels

#### 3. Enhanced Endpoint Handler (`src/endpoints/github/repo_stars/read_daily_graph/index.rs`)
- **Robust validation**: Input validation with clear error messages
- **Flexible request structure**: Support for multiple repositories and configurations
- **Clean error handling**: Comprehensive error types with proper HTTP status codes

## Smart Time Scale System

The relative time axis automatically chooses appropriate intervals based on the total data duration:

| Duration | Interval | Example Labels |
|----------|----------|----------------|
| ≤ 2 months | Every 7 days | 0, 7d, 14d, 21d, 28d |
| ≤ 1 year | Every month | 0, 1m, 2m, 3m, 4m |
| ≤ 3 years | Every 3 months | 0, 3m, 6m, 9m, 1y |
| ≤ 10 years | Every year | 0, 1y, 2y, 3y, 4y |
| > 10 years | Every 2 years | 0, 2y, 4y, 6y, 8y |

This ensures clean, readable charts without cluttered labels like "1y4m".

## API Usage

### Request Format

```json
{
  "repositories": [
    {"owner": "facebook", "name": "react"},
    {"owner": "microsoft", "name": "vscode"}
  ],
  "metric_types": ["position", "speed"],
  "relative_x_axis": true,
  "chart_config": {
    "width": 1000,
    "height": 600,
    "title": "React vs VSCode Star Growth",
    "show_legend": true
  }
}
```

### Request Parameters

#### `repositories` (required)
Array of repository specifications:
- `owner`: Repository owner (string)
- `name`: Repository name (string)
- Maximum 10 repositories per request

#### `metric_types` (optional, default: ["position"])
Array of metric types to calculate:
- `"position"`: Cumulative star count over time
- `"speed"`: Daily star count (first derivative)
- `"acceleration"`: Change in daily star count (second derivative)

#### `relative_x_axis` (optional, default: false)
- `true`: X-axis shows relative time with smart intervals (0, 7d, 1m, 1y, etc.) from earliest data point
- `false`: X-axis shows absolute dates

#### `chart_config` (optional)
Chart appearance configuration:
- `width`: Chart width in pixels (default: 800)
- `height`: Chart height in pixels (default: 400)
- `title`: Custom chart title (auto-generated if not provided)
- `show_legend`: Whether to show legend (default: true)

### Response

Returns an SVG chart as `image/svg+xml` with appropriate caching headers.

## Example Use Cases

### 1. Single Repository Growth Analysis
```json
{
  "repositories": [{"owner": "facebook", "name": "react"}],
  "metric_types": ["position", "speed", "acceleration"],
  "relative_x_axis": false
}
```
Shows React's cumulative stars, daily growth rate, and growth acceleration over actual dates.

### 2. Multi-Repository Comparison (Relative Timeline)
```json
{
  "repositories": [
    {"owner": "facebook", "name": "react"},
    {"owner": "angular", "name": "angular"},
    {"owner": "vuejs", "name": "vue"}
  ],
  "metric_types": ["position"],
  "relative_x_axis": true
}
```
Compares framework adoption starting from each project's first star (time 0).

### 3. Growth Velocity Analysis
```json
{
  "repositories": [
    {"owner": "microsoft", "name": "vscode"},
    {"owner": "atom", "name": "atom"}
  ],
  "metric_types": ["speed"],
  "relative_x_axis": true,
  "chart_config": {
    "title": "Editor Growth Velocity Comparison",
    "width": 1200,
    "height": 800
  }
}
```
Analyzes daily star acquisition rates for competing editors.

## Mathematical Definitions

### Position (Cumulative Stars)
```
position(t) = Σ(daily_stars[0..t])
```
Total stars accumulated up to time t.

### Speed (Daily Star Rate)
```
speed(t) = daily_stars[t]
```
Number of stars gained on day t.

### Acceleration (Growth Rate Change)
```
acceleration(t) = speed(t) - speed(t-1)
```
Change in daily star acquisition rate.

## Error Handling

The endpoint provides comprehensive error handling:

- **400 Bad Request**: Invalid input (empty repositories, invalid metric types)
- **404 Not Found**: Repository not found in database
- **500 Internal Server Error**: Database errors, data processing failures, chart generation errors

## Performance Considerations

- **Repository Limit**: Maximum 10 repositories per request to prevent performance issues
- **Caching**: SVG responses are cached for 1 hour
- **Efficient Processing**: Data is processed in batches and mathematical operations are optimized

## Future Enhancements

Potential improvements to consider:

1. **Multiple Chart Support**: Return multiple SVGs for different metric types
2. **Export Formats**: Support PNG, PDF export options
3. **Time Range Filtering**: Allow filtering by date ranges
4. **Statistical Analysis**: Add trend lines, correlation analysis
5. **Real-time Updates**: WebSocket support for live chart updates
6. **Custom Aggregations**: Weekly, monthly aggregation options 