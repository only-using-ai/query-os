# ORDER BY Implementation Scratchpad

## Task: Implement ORDER BY with ASC/DESC support

### Current State Analysis
- ORDER BY parsing exists but only supports field name, no ASC/DESC
- Sorting functions exist for filesystem, network, and process queries
- All sorting currently defaults to ascending order only

### Requirements
- Support `ORDER BY field ASC` and `ORDER BY field DESC`
- ASC should be default (maintain backward compatibility)
- Work with all query types: filesystem, network, processes
- Support the example: `SELECT DISTINCT port FROM net WHERE port NOT NULL ORDER BY port DESC`

### Implementation Plan
1. Update SqlQuery model to include sort direction
2. Update Pest grammar for ASC/DESC keywords
3. Update parser logic to capture sort direction
4. Update all sorting functions to support DESC ordering
5. Add comprehensive tests

### Decisions Made
- ASC will be default behavior (no keyword needed)
- DESC requires explicit DESC keyword
- Sort direction stored as enum: Ascending, Descending

### Progress Tracking
- [x] Create SCRATCHPAD.md
- [x] Update SqlQuery model
- [x] Update Pest grammar
- [x] Update parser logic
- [x] Update sorting functions
- [x] Add unit tests
- [x] Test example query

### Notes
- Need to maintain backward compatibility
- All existing ORDER BY queries should continue working (default to ASC)
- Focus on the network query example provided