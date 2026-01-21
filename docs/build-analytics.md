# Build Analytics

## Overview

CCGO provides **build analytics** to track and analyze build performance over time. This helps identify bottlenecks, track improvements, and optimize build configurations.

## Benefits

- 📊 **Performance Tracking** - Monitor build times and identify trends
- 🎯 **Bottleneck Identification** - Find which phases take longest
- 📈 **Cache Effectiveness** - Track compiler cache hit/miss rates
- 🔍 **Build History** - Review past builds and compare performance
- 📉 **Optimization Insights** - Data-driven build optimization decisions

## Quick Start

Build analytics are **automatically collected** during builds and stored locally in `~/.ccgo/analytics/`.

### View Analytics

```bash
# Show recent builds (default: 10)
ccgo analytics show

# Show more builds
ccgo analytics show -n 20

# Show summary statistics
ccgo analytics summary

# List all projects with analytics
ccgo analytics list
```

## Commands

### `ccgo analytics show`

Display recent build history with key metrics:

```bash
$ ccgo analytics show

================================================================================
Build Analytics for myproject
================================================================================

Build #1 - linux (2026-01-21T15:30:00+08:00)
  Duration:    45.30s
  Jobs:        8
  Success:     ✓
  Cache Tool:  sccache
  Cache Rate:  78.5%

Build #2 - linux (2026-01-21T16:00:00+08:00)
  Duration:    12.50s
  Jobs:        8
  Success:     ✓
  Cache Tool:  sccache
  Cache Rate:  95.2%

Build #3 - macos (2026-01-21T16:15:00+08:00)
  Duration:    38.70s
  Jobs:        8
  Success:     ✓
  Cache Tool:  ccache
  Cache Rate:  82.3%
```

**Options**:
- `-n, --count <NUM>` - Number of builds to show (default: 10)

### `ccgo analytics summary`

Show aggregate statistics across all builds:

```bash
$ ccgo analytics summary

================================================================================
Build Analytics Summary for myproject
================================================================================

Total Builds:      25
Successful:        24 (96.0%)

Build Duration:
  Average:         32.45s
  Fastest:         11.20s
  Slowest:         58.90s

Cache Statistics:
  Builds with cache: 23
  Avg Hit Rate:      85.3%

Platform Breakdown:
  linux............... 15
  macos............... 8
  ios................. 2

================================================================================
```

### `ccgo analytics clear`

Clear analytics history for the current project:

```bash
$ ccgo analytics clear

This will delete 25 build analytics entries for 'myproject'
Continue? [y/N] y
✓ Cleared analytics for 'myproject'
```

**Options**:
- `-y, --yes` - Skip confirmation prompt

### `ccgo analytics export`

Export analytics data to JSON file:

```bash
$ ccgo analytics export -o builds.json

✓ Exported 25 build analytics to builds.json
```

**Options**:
- `-o, --output <FILE>` - Output file path

### `ccgo analytics list`

List all projects with analytics data:

```bash
$ ccgo analytics list

================================================================================
Projects with Analytics
================================================================================

  myproject.................................... 25 builds
  another-lib.................................. 12 builds
  experimental................................. 5 builds

Use 'ccgo analytics show' from a project directory to view details.
================================================================================
```

## Collected Metrics

### Build Overview

- **Project Name** - Project being built
- **Platform** - Target platform (linux, macos, windows, etc.)
- **Timestamp** - When build started (ISO 8601)
- **Total Duration** - Complete build time in seconds
- **Parallel Jobs** - Number of parallel compilation jobs
- **Success Status** - Whether build succeeded
- **Errors/Warnings** - Count of compilation diagnostics

### Phase Breakdown

Build is divided into phases with timing for each:

1. **Dependency Resolution** - Installing and resolving dependencies
2. **CMake Configuration** - CMake configure step
3. **Compilation** - C/C++ source compilation
4. **Linking** - Linking libraries
5. **Archiving** - Creating ZIP archives
6. **Post-Processing** - Additional packaging steps

Each phase tracks:
- Duration in seconds
- Percentage of total build time

### Cache Statistics

For builds using ccache/sccache:

- **Cache Tool** - Which tool is being used (ccache, sccache)
- **Cache Hits** - Compilation artifacts reused from cache
- **Cache Misses** - New compilation results added to cache
- **Hit Rate** - Percentage of cache hits (0-100%)

### File Statistics

- **Source Files** - Number of .c/.cc/.cpp files
- **Header Files** - Number of .h/.hpp files
- **Total Lines** - Combined lines of code
- **Artifact Size** - Final output size in bytes

## Data Storage

Analytics data is stored locally in:

```
~/.ccgo/analytics/
├── myproject.json      # Analytics for myproject
├── another-lib.json    # Analytics for another-lib
└── ...
```

Each project file contains:
- Last 100 builds (older builds are automatically pruned)
- JSON format for easy parsing and export
- No personally identifiable information

## Analytics API (Rust)

For programmatic access within CCGO:

```rust
use ccgo::build::analytics::{BuildAnalytics, AnalyticsCollector, BuildPhase};

// Create collector
let mut collector = AnalyticsCollector::new(
    "myproject".to_string(),
    "linux".to_string(),
    8, // parallel jobs
);

// Time phases
collector.start_phase(BuildPhase::Compilation);
// ... compilation work ...
collector.end_phase(BuildPhase::Compilation);

// Record diagnostics
collector.add_diagnostics(2, 15); // 2 errors, 15 warnings

// Set success status
collector.set_success(true);

// Finalize and save
let analytics = collector.finalize(cache_stats, file_stats);
analytics.save()?;

// Load history
let history = BuildAnalytics::load_history("myproject")?;

// Get average build time
let avg = BuildAnalytics::average_build_time("myproject")?;
```

## Use Cases

### Performance Regression Detection

```bash
# After making changes
ccgo build linux

# Check if build got slower
ccgo analytics summary

# Expected: Average duration should not increase significantly
```

### Cache Effectiveness

```bash
# First build (cold cache)
ccgo build linux --cache sccache
# Note the duration

# Second build (warm cache)
ccgo build linux --cache sccache
# Should be 50-80% faster

# Check cache hit rate
ccgo analytics show -n 1
# Expected: High cache hit rate (>80%)
```

### CI/CD Monitoring

```yaml
# .github/workflows/build.yml
- name: Build
  run: ccgo build linux

- name: Show Analytics
  run: ccgo analytics show -n 1

- name: Export Analytics
  run: ccgo analytics export -o build-stats.json

- name: Upload Analytics
  uses: actions/upload-artifact@v3
  with:
    name: build-analytics
    path: build-stats.json
```

### Comparing Platforms

```bash
# Build multiple platforms
ccgo build linux
ccgo build macos
ccgo build windows

# View summary
ccgo analytics summary

# Platform Breakdown shows relative performance
```

## Best Practices

### DO

✅ **Review regularly** - Check analytics after significant changes
✅ **Track trends** - Monitor if builds are getting slower over time
✅ **Optimize hot paths** - Focus on phases with highest percentage
✅ **Use caching** - Compiler cache dramatically improves metrics
✅ **Export for reports** - Use JSON export for trend analysis

### DON'T

❌ **Don't commit analytics** - Data is local to your machine
❌ **Don't manually edit** - Analytics files are auto-generated
❌ **Don't rely on first build** - Cold cache builds are always slower
❌ **Don't compare different machines** - Hardware affects timing

## Troubleshooting

### No Analytics Data

**Symptom**: `ccgo analytics show` says "No build analytics available"

**Solution**: Analytics collection is automatic but requires:
```bash
# Run a build first
ccgo build linux

# Then check analytics
ccgo analytics show
```

### Analytics Not Updating

**Symptom**: New builds don't appear in analytics

**Solution**: Check that builds are completing successfully:
```bash
# Verify build succeeds
ccgo build linux

# Check recent analytics
ccgo analytics show -n 1
```

### Wrong Project Analytics

**Symptom**: Seeing analytics from different project

**Solution**: Analytics are tied to project name in `CCGO.toml`:
```bash
# Check current project
grep "name =" CCGO.toml

# Make sure you're in correct directory
pwd
```

### Storage Location

Analytics are stored in:
- **Linux**: `~/.ccgo/analytics/`
- **macOS**: `~/.ccgo/analytics/`
- **Windows**: `%USERPROFILE%\.ccgo\analytics\`

## Future Enhancements

Planned features for future releases:

- [ ] Real-time build progress visualization
- [ ] Memory usage tracking
- [ ] Dependency compilation breakdown
- [ ] Historical trend graphs
- [ ] Export to CSV/Excel
- [ ] Integration with build dashboards
- [ ] Comparative analysis across teams
- [ ] Automatic regression alerts

## See Also

- [Build Caching](build-caching.md) - Improve build times with caching
- [Build System](features/build-system.md) - General build system overview
- [Incremental Builds](incremental-builds.md) - Faster rebuild strategies

## Changelog

### v3.0.11 (2026-01-21)

- ✅ Implemented build analytics system
- ✅ Added `ccgo analytics` command with show/summary/clear/export/list subcommands
- ✅ Automatic collection of build metrics
- ✅ Phase timing breakdown
- ✅ Cache statistics integration
- ✅ File and error/warning tracking
- ✅ Local storage in `~/.ccgo/analytics/`
- ✅ Last 100 builds per project (auto-pruning)

---

*Build analytics help you make data-driven decisions about build optimization and configuration.*
