//! Spatial table detection from PDF text layout.
//!
//! Implements table detection according to ISO 32000-1:2008 Section 5.2 (Coordinate Systems).
//! Uses X and Y coordinate clustering to identify table structure in PDFs that lack explicit
//! table markup in the structure tree.

use crate::layout::text_block::TextSpan;
use crate::structure::table_extractor::{ExtractedTable, TableCell, TableRow};
use std::collections::HashMap;

/// Strategy for detecting table boundaries (v0.3.14).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum TableStrategy {
    /// Use only vector lines to define boundaries.
    #[serde(rename = "lines")]
    Lines,
    /// Use only text alignment to define boundaries.
    #[serde(rename = "text")]
    Text,
    /// Use both text and lines (hybrid approach).
    #[default]
    #[serde(rename = "both")]
    Both,
}

/// Configuration for spatial table detection.
#[derive(Debug, Clone, PartialEq)]
pub struct TableDetectionConfig {
    /// Whether table detection is enabled.
    pub enabled: bool,
    /// Strategy for horizontal boundary detection.
    pub horizontal_strategy: TableStrategy,
    /// Strategy for vertical boundary detection.
    pub vertical_strategy: TableStrategy,
    /// X-coordinate tolerance for column grouping.
    pub column_tolerance: f32,
    /// Y-coordinate tolerance for row grouping.
    pub row_tolerance: f32,
    /// Minimum number of cells required for a valid table.
    pub min_table_cells: usize,
    /// Minimum number of columns required for a valid table.
    pub min_table_columns: usize,
    /// Ratio of regular rows required for a valid table structure.
    pub regular_row_ratio: f32,
    /// Maximum number of columns allowed before rejecting as false positive.
    pub max_table_columns: usize,
}

impl Default for TableDetectionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            horizontal_strategy: TableStrategy::Both,
            vertical_strategy: TableStrategy::Both,
            column_tolerance: 5.0,
            row_tolerance: 2.8,
            min_table_cells: 4,
            min_table_columns: 2,
            regular_row_ratio: 0.3,
            max_table_columns: 15,
        }
    }
}

impl TableDetectionConfig {
    /// Create a strict table detection configuration.
    pub fn strict() -> Self {
        Self {
            enabled: true,
            horizontal_strategy: TableStrategy::Lines,
            vertical_strategy: TableStrategy::Lines,
            column_tolerance: 2.0,
            row_tolerance: 1.0,
            min_table_cells: 6,
            min_table_columns: 3,
            regular_row_ratio: 0.8,
            max_table_columns: 12,
        }
    }

    /// Create a relaxed table detection configuration.
    pub fn relaxed() -> Self {
        Self {
            enabled: true,
            horizontal_strategy: TableStrategy::Text,
            vertical_strategy: TableStrategy::Text,
            column_tolerance: 10.0,
            row_tolerance: 5.0,
            min_table_cells: 4,
            min_table_columns: 2,
            regular_row_ratio: 0.3,
            max_table_columns: 20,
        }
    }
}

/// Detect tables from spatial layout of text spans.
pub fn detect_tables_from_spans(
    spans: &[TextSpan],
    config: &TableDetectionConfig,
) -> Vec<ExtractedTable> {
    if !config.enabled || spans.is_empty() {
        return Vec::new();
    }

    let columns = detect_columns(spans, config.column_tolerance);
    if columns.len() < config.min_table_columns.max(2) || columns.len() > config.max_table_columns {
        return Vec::new();
    }

    let rows = detect_rows(spans, config.row_tolerance);
    if rows.len() < 2 {
        return Vec::new();
    }

    let grid = assign_spans_to_cells(spans, &columns, &rows);
    if !validate_table_structure_internal(&grid, config) {
        return Vec::new();
    }

    vec![grid_to_extracted_table(&grid, spans, None)]
}

#[derive(Debug, Clone)]
struct ColumnCluster {
    x_center: f32,
    x_min: f32,
    x_max: f32,
    span_indices: Vec<usize>,
}

#[derive(Debug, Clone)]
struct RowCluster {
    y_center: f32,
    y_min: f32,
    y_max: f32,
    span_indices: Vec<usize>,
}

#[derive(Debug, Clone)]
struct GridStructure {
    columns: Vec<ColumnCluster>,
    rows: Vec<RowCluster>,
    cells: Vec<Vec<Vec<usize>>>,
}

impl GridStructure {
    fn is_row_empty(&self, row_idx: usize) -> bool {
        self.cells[row_idx].iter().all(|cell| cell.is_empty())
    }

    fn is_column_empty(&self, col_idx: usize) -> bool {
        for row in &self.cells {
            if !row[col_idx].is_empty() {
                return false;
            }
        }
        true
    }

    fn trim_empty_columns(&self) -> GridStructure {
        let num_rows = self.cells.len();
        let num_cols = self.columns.len();

        let mut first_col = 0;
        while first_col < num_cols && self.is_column_empty(first_col) {
            first_col += 1;
        }

        let mut last_col = num_cols;
        while last_col > first_col && self.is_column_empty(last_col - 1) {
            last_col -= 1;
        }

        if first_col >= last_col {
            return self.clone();
        }

        let mut active_cols = Vec::new();
        for c in first_col..last_col {
            let col_width = self.columns[c].x_max - self.columns[c].x_min;
            if col_width < 2.0 && self.is_column_empty(c) {
                continue;
            }
            active_cols.push(c);
        }

        if active_cols.is_empty() {
            return self.clone();
        }

        let new_columns: Vec<ColumnCluster> = active_cols
            .iter()
            .map(|&c| self.columns[c].clone())
            .collect();

        let mut new_cells = Vec::with_capacity(num_rows);
        for r in 0..num_rows {
            let row_cells = active_cols
                .iter()
                .map(|&c| self.cells[r][c].clone())
                .collect();
            new_cells.push(row_cells);
        }

        GridStructure {
            columns: new_columns,
            rows: self.rows.clone(),
            cells: new_cells,
        }
    }
}

#[derive(Debug, Clone)]
struct CellMergeInfo {
    colspan: u32,
    rowspan: u32,
    covered: bool,
}

fn detect_columns(spans: &[TextSpan], column_tolerance: f32) -> Vec<ColumnCluster> {
    let mut columns: Vec<ColumnCluster> = Vec::new();
    for (idx, span) in spans.iter().enumerate() {
        let x = span.bbox.left();
        let mut found = false;
        for col in &mut columns {
            if (x - col.x_center).abs() < column_tolerance {
                col.span_indices.push(idx);
                col.x_min = col.x_min.min(x);
                col.x_max = col.x_max.max(x);
                found = true;
                break;
            }
        }
        if !found {
            columns.push(ColumnCluster {
                x_center: x,
                x_min: x,
                x_max: x,
                span_indices: vec![idx],
            });
        }
    }
    columns.sort_by(|a, b| crate::utils::safe_float_cmp(a.x_center, b.x_center));
    columns
}

fn detect_rows(spans: &[TextSpan], row_tolerance: f32) -> Vec<RowCluster> {
    let mut rows: Vec<RowCluster> = Vec::new();
    for (idx, span) in spans.iter().enumerate() {
        let y = span.bbox.center().y;
        let mut found = false;
        for row in &mut rows {
            if (y - row.y_center).abs() < row_tolerance {
                row.span_indices.push(idx);
                row.y_min = row.y_min.min(y);
                row.y_max = row.y_max.max(y);
                found = true;
                break;
            }
        }
        if !found {
            rows.push(RowCluster {
                y_center: y,
                y_min: y,
                y_max: y,
                span_indices: vec![idx],
            });
        }
    }
    rows.sort_by(|a, b| crate::utils::safe_float_cmp(b.y_center, a.y_center));
    rows
}

fn assign_spans_to_cells(
    spans: &[TextSpan],
    columns: &[ColumnCluster],
    rows: &[RowCluster],
) -> GridStructure {
    let num_cols = columns.len();
    let num_rows = rows.len();
    let mut cells: Vec<Vec<Vec<usize>>> = vec![vec![Vec::new(); num_cols]; num_rows];
    for (idx, span) in spans.iter().enumerate() {
        let span_x = span.bbox.center().x;
        let span_y = span.bbox.center().y;
        let col_idx = columns
            .iter()
            .enumerate()
            .min_by_key(|(_, col)| ((span_x - col.x_center).abs() * 1000.0) as i32)
            .map(|(i, _)| i)
            .unwrap_or(0);
        let row_idx = rows
            .iter()
            .enumerate()
            .min_by_key(|(_, row)| ((span_y - row.y_center).abs() * 1000.0) as i32)
            .map(|(i, _)| i)
            .unwrap_or(0);
        cells[row_idx][col_idx].push(idx);
    }
    GridStructure {
        columns: columns.to_vec(),
        rows: rows.to_vec(),
        cells,
    }
}

fn validate_table_structure_internal(grid: &GridStructure, config: &TableDetectionConfig) -> bool {
    let total_cells: usize = grid
        .cells
        .iter()
        .flat_map(|row| row.iter())
        .map(|cell| if cell.is_empty() { 0 } else { 1 })
        .sum();
    if total_cells < config.min_table_cells {
        return false;
    }
    let cell_counts: Vec<usize> = grid
        .cells
        .iter()
        .map(|row| row.iter().filter(|cell| !cell.is_empty()).count())
        .collect();
    if cell_counts.is_empty() {
        return false;
    }
    let most_common_count = *cell_counts
        .iter()
        .max_by_key(|&&count| cell_counts.iter().filter(|&&c| c == count).count())
        .unwrap_or(&0);
    if most_common_count == 0 {
        return false;
    }
    let regular_rows = cell_counts
        .iter()
        .filter(|&&count| count == most_common_count)
        .count();
    (regular_rows as f32 / cell_counts.len() as f32) >= config.regular_row_ratio
}

/// Backward compatibility: Indices of spans belonging to a table.
#[derive(Debug, Clone)]
pub struct DetectedTable {
    /// Indices of spans that belong to this table.
    pub span_indices: Vec<usize>,
}

/// Backward compatibility: Table detector wrapper.
pub struct SpatialTableDetector {
    /// Configuration for this detector.
    pub config: TableDetectionConfig,
}

impl SpatialTableDetector {
    /// Create a new detector with config.
    pub fn with_config(config: TableDetectionConfig) -> Self {
        Self { config }
    }
    /// Detect tables (wrapper).
    pub fn detect_tables(&self, spans: &[TextSpan]) -> Vec<DetectedTable> {
        detect_tables_from_spans(spans, &self.config)
            .into_iter()
            .flat_map(|_| None)
            .collect()
    }
    /// Detect tables using visual lines and text (hybrid).
    pub fn detect_tables_hybrid(
        &self,
        spans: &[TextSpan],
        lines: &[crate::elements::PathContent],
    ) -> Vec<ExtractedTable> {
        detect_tables_with_lines(spans, lines, &self.config)
    }
}

fn cluster_values(values: &[f32], tolerance: f32) -> Vec<f32> {
    let mut clusters: Vec<f32> = Vec::new();
    let mut counts: Vec<u32> = Vec::new();
    for &v in values {
        if let Some(idx) = clusters.iter().position(|&c| (v - c).abs() < tolerance) {
            counts[idx] += 1;
            clusters[idx] += (v - clusters[idx]) / counts[idx] as f32;
        } else {
            clusters.push(v);
            counts.push(1);
        }
    }
    clusters
}

struct LineCluster {
    lines: Vec<usize>,
    bbox: crate::geometry::Rect,
}

impl LineCluster {
    fn new(line_idx: usize, bbox: crate::geometry::Rect) -> Self {
        Self {
            lines: vec![line_idx],
            bbox,
        }
    }
    fn add(&mut self, line_idx: usize, bbox: crate::geometry::Rect) {
        self.lines.push(line_idx);
        self.bbox = self.bbox.union(&bbox);
    }
}

fn group_lines_into_clusters(lines: &[crate::elements::PathContent]) -> Vec<LineCluster> {
    if lines.is_empty() {
        return Vec::new();
    }
    let mut parent: Vec<usize> = (0..lines.len()).collect();
    fn find(i: usize, parent: &mut [usize]) -> usize {
        let mut curr = i;
        while parent[curr] != curr {
            parent[curr] = parent[parent[curr]];
            curr = parent[curr];
        }
        curr
    }
    fn union(i: usize, j: usize, parent: &mut [usize]) {
        let root_i = find(i, parent);
        let root_j = find(j, parent);
        if root_i != root_j {
            parent[root_i] = root_j;
        }
    }
    let mut valid_indices: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, path)| path.is_table_primitive())
        .map(|(i, _)| i)
        .collect();

    // Optimization: Sort by X-coordinate to enable sweep-line early exit (O(n log n))
    valid_indices.sort_by(|&a, &b| crate::utils::safe_float_cmp(lines[a].bbox.x, lines[b].bbox.x));

    const EXPANSION: f32 = 10.0;
    for i in 0..valid_indices.len() {
        let idx_a = valid_indices[i];
        let bbox_a = &lines[idx_a].bbox;
        let expanded_a = crate::geometry::Rect::new(
            bbox_a.x - EXPANSION,
            bbox_a.y - EXPANSION,
            bbox_a.width + EXPANSION * 2.0,
            bbox_a.height + EXPANSION * 2.0,
        );

        for j in (i + 1)..valid_indices.len() {
            let idx_b = valid_indices[j];
            let bbox_b = &lines[idx_b].bbox;

            // Optimization: If the next path's X-start is beyond our search threshold,
            // no subsequent paths in the sorted list can possibly intersect.
            if bbox_b.x > expanded_a.x + expanded_a.width {
                break;
            }

            let expanded_b = crate::geometry::Rect::new(
                bbox_b.x - EXPANSION,
                bbox_b.y - EXPANSION,
                bbox_b.width + EXPANSION * 2.0,
                bbox_b.height + EXPANSION * 2.0,
            );

            if expanded_a.intersects(&expanded_b) {
                union(idx_a, idx_b, &mut parent);
            }
        }
    }
    let mut cluster_map: HashMap<usize, LineCluster> = HashMap::new();
    for i in valid_indices {
        let root = find(i, &mut parent);
        let bbox = lines[i].bbox;
        cluster_map
            .entry(root)
            .and_modify(|c| c.add(i, bbox))
            .or_insert_with(|| LineCluster::new(i, bbox));
    }
    cluster_map.into_values().collect()
}

fn detect_tables_in_cluster(
    spans: &[TextSpan],
    all_lines: &[crate::elements::PathContent],
    cluster: &LineCluster,
    config: &TableDetectionConfig,
) -> Vec<ExtractedTable> {
    const MIN_LINE_LENGTH: f32 = 5.0;
    const LINE_AXIS_TOL: f32 = 2.0;
    let mut h_ys: Vec<f32> = Vec::new();
    let mut v_xs: Vec<f32> = Vec::new();
    for &idx in &cluster.lines {
        let path = &all_lines[idx];
        let bbox = &path.bbox;
        if path.is_horizontal_line(LINE_AXIS_TOL) && bbox.width > MIN_LINE_LENGTH {
            h_ys.push(bbox.center().y);
        }
        if path.is_vertical_line(LINE_AXIS_TOL) && bbox.height.abs() > MIN_LINE_LENGTH {
            v_xs.push(bbox.center().x);
        }
    }
    let mut row_ys = cluster_values(&h_ys, config.row_tolerance);
    let mut col_xs = cluster_values(&v_xs, config.column_tolerance);
    if row_ys.len() < 2 || col_xs.len() < 2 {
        return Vec::new();
    }
    row_ys.sort_by(|a, b| crate::utils::safe_float_cmp(*b, *a));
    col_xs.sort_by(|a, b| crate::utils::safe_float_cmp(*a, *b));
    let num_rows = row_ys.len() - 1;
    let num_cols = col_xs.len() - 1;
    if num_cols < config.min_table_columns || num_cols > config.max_table_columns {
        return Vec::new();
    }
    let mut cells: Vec<Vec<Vec<usize>>> = vec![vec![Vec::new(); num_cols]; num_rows];
    let mut assigned_any = false;
    for (orig_idx, span) in spans.iter().enumerate() {
        if !cluster.bbox.intersects(&span.bbox) {
            continue;
        }
        let cx = span.bbox.center().x;
        let cy = span.bbox.center().y;
        let row_idx = (0..num_rows).find(|&r| cy <= row_ys[r] && cy >= row_ys[r + 1]);
        let col_idx = (0..num_cols).find(|&c| cx >= col_xs[c] && cx <= col_xs[c + 1]);
        if let (Some(r), Some(c)) = (row_idx, col_idx) {
            cells[r][c].push(orig_idx);
            assigned_any = true;
        }
    }
    if !assigned_any {
        return Vec::new();
    }
    let columns: Vec<ColumnCluster> = (0..num_cols)
        .map(|c| ColumnCluster {
            x_center: (col_xs[c] + col_xs[c + 1]) / 2.0,
            x_min: col_xs[c],
            x_max: col_xs[c + 1],
            span_indices: Vec::new(),
        })
        .collect();
    let all_rows: Vec<RowCluster> = (0..num_rows)
        .map(|r| RowCluster {
            y_center: (row_ys[r] + row_ys[r + 1]) / 2.0,
            y_min: row_ys[r + 1],
            y_max: row_ys[r],
            span_indices: Vec::new(),
        })
        .collect();
    let grid_full = GridStructure {
        columns: columns.clone(),
        rows: all_rows.clone(),
        cells: cells.clone(),
    };
    let mut tables = Vec::new();
    let mut current_start_row = 0;
    while current_start_row < num_rows {
        if grid_full.is_row_empty(current_start_row) {
            current_start_row += 1;
            continue;
        }
        let mut current_end_row = current_start_row;
        while current_end_row < num_rows {
            if grid_full.is_row_empty(current_end_row) {
                break;
            }
            current_end_row += 1;
        }
        if current_end_row > current_start_row {
            let sub_cells = cells[current_start_row..current_end_row].to_vec();
            let sub_rows = all_rows[current_start_row..current_end_row].to_vec();
            let mut grid = GridStructure {
                columns: columns.clone(),
                rows: sub_rows,
                cells: sub_cells,
            };
            grid = grid.trim_empty_columns();
            if validate_table_structure_internal(&grid, config) {
                let mut table = grid_to_extracted_table(
                    &grid,
                    spans,
                    Some(detect_merged_cells_visually(&grid, spans, cluster, all_lines)),
                );
                let mut min_y = f32::INFINITY;
                let mut max_y = f32::NEG_INFINITY;
                for r in &grid.rows {
                    min_y = min_y.min(r.y_min);
                    max_y = max_y.max(r.y_max);
                }
                table.bbox = Some(crate::geometry::Rect::new(
                    cluster.bbox.x,
                    min_y,
                    cluster.bbox.width,
                    max_y - min_y,
                ));
                let mut header_rows_detected = 0;
                let table_width = cluster.bbox.width;
                for r in 0..table.rows.len().min(3) {
                    let row_bottom = grid.rows[r].y_min;
                    let has_separator = cluster.lines.iter().any(|&idx| {
                        let path = &all_lines[idx];
                        path.is_horizontal_line(LINE_AXIS_TOL)
                            && path.bbox.width > table_width * 0.8
                            && (path.bbox.center().y - row_bottom).abs() < config.row_tolerance
                    });
                    if has_separator {
                        header_rows_detected = r + 1;
                    } else if r == 0 && table.rows[r].has_colspan() {
                        header_rows_detected = 1;
                    } else {
                        break;
                    }
                }
                if header_rows_detected > 0 {
                    table.has_header = true;
                    for r in 0..header_rows_detected {
                        if r < table.rows.len() {
                            table.rows[r].is_header = true;
                            for cell in &mut table.rows[r].cells {
                                cell.is_header = true;
                            }
                        }
                    }
                }
                tables.push(table);
            }
        }
        current_start_row = current_end_row + 1;
    }
    tables
}

fn detect_merged_cells_visually(
    grid: &GridStructure,
    spans: &[TextSpan],
    cluster: &LineCluster,
    all_lines: &[crate::elements::PathContent],
) -> Vec<Vec<CellMergeInfo>> {
    let num_rows = grid.cells.len();
    let num_cols = grid.columns.len();
    const LINE_TOLERANCE: f32 = 2.0;
    let mut merge_info: Vec<Vec<CellMergeInfo>> = (0..num_rows)
        .map(|_| {
            (0..num_cols)
                .map(|_| CellMergeInfo {
                    colspan: 1,
                    rowspan: 1,
                    covered: false,
                })
                .collect()
        })
        .collect();
    for r in 0..num_rows {
        let mut c = 0;
        while c < num_cols {
            if merge_info[r][c].covered {
                c += 1;
                continue;
            }
            let mut colspan = 1;
            let mut cell_text_width: f32 = 0.0;
            for &idx in &grid.cells[r][c] {
                cell_text_width = cell_text_width.max(spans[idx].bbox.width);
            }
            let mut total_cell_width = grid.columns[c].x_max - grid.columns[c].x_min;
            for next_c in (c + 1)..num_cols {
                let separator_x = grid.columns[next_c].x_min;
                let y_min = grid.rows[r].y_min;
                let y_max = grid.rows[r].y_max;
                let has_separator = cluster.lines.iter().any(|&idx| {
                    let path = &all_lines[idx];
                    path.is_vertical_line(LINE_TOLERANCE)
                        && (path.bbox.center().x - separator_x).abs() < LINE_TOLERANCE
                        && path.bbox.y < y_max
                        && (path.bbox.y + path.bbox.height) > y_min
                });
                if !has_separator || (cell_text_width > total_cell_width + 2.0) {
                    colspan += 1;
                    total_cell_width += grid.columns[next_c].x_max - grid.columns[next_c].x_min;
                } else {
                    break;
                }
            }
            if colspan > 1 {
                merge_info[r][c].colspan = colspan;
                for i in 1..colspan {
                    merge_info[r][c + i as usize].covered = true;
                }
            }
            c += colspan as usize;
        }
    }
    for c in 0..num_cols {
        let mut r = 0;
        while r < num_rows {
            if merge_info[r][c].covered {
                r += 1;
                continue;
            }
            let mut rowspan = 1;
            let current_colspan = merge_info[r][c].colspan;
            for next_r in (r + 1)..num_rows {
                let separator_y = grid.rows[next_r].y_max;
                let x_min = grid.columns[c].x_min;
                let x_max = grid.columns[c + current_colspan as usize - 1].x_max;
                let has_separator = cluster.lines.iter().any(|&idx| {
                    let path = &all_lines[idx];
                    path.is_horizontal_line(LINE_TOLERANCE)
                        && (path.bbox.center().y - separator_y).abs() < LINE_TOLERANCE
                        && path.bbox.x < x_max
                        && (path.bbox.x + path.bbox.width) > x_min
                });
                if !has_separator {
                    rowspan += 1;
                } else {
                    break;
                }
            }
            if rowspan > 1 {
                merge_info[r][c].rowspan = rowspan;
                for i in 1..rowspan {
                    merge_info[r + i as usize][c].covered = true;
                    for j in 1..current_colspan {
                        merge_info[r + i as usize][c + j as usize].covered = true;
                    }
                }
            }
            r += rowspan as usize;
        }
    }
    merge_info
}

/// Detect tables using vector lines and text spans (main entry point for hybrid detection).
pub fn detect_tables_with_lines(
    spans: &[TextSpan],
    lines: &[crate::elements::PathContent],
    config: &TableDetectionConfig,
) -> Vec<ExtractedTable> {
    if !config.enabled || spans.is_empty() {
        return Vec::new();
    }
    match (config.horizontal_strategy, config.vertical_strategy) {
        (TableStrategy::Text, TableStrategy::Text) => {
            return detect_tables_from_spans(spans, config)
        },
        (TableStrategy::Lines, TableStrategy::Lines) => {
            let clusters = group_lines_into_clusters(lines);
            let mut tables = Vec::new();
            for cluster in clusters {
                tables.append(&mut detect_tables_in_cluster(spans, lines, &cluster, config));
            }
            return tables;
        },
        _ => {},
    }
    let clusters = group_lines_into_clusters(lines);
    let mut final_tables = Vec::new();
    for cluster in clusters {
        final_tables.append(&mut detect_tables_in_cluster(spans, lines, &cluster, config));
    }
    let text_candidates = detect_tables_from_spans(spans, config);
    for text_table in text_candidates {
        if let Some(text_bbox) = text_table.bbox {
            let overlaps = final_tables.iter().any(|t| {
                if let Some(line_bbox) = t.bbox {
                    line_bbox.intersects(&text_bbox)
                        || line_bbox.contains_rect(&text_bbox)
                        || text_bbox.contains_rect(&line_bbox)
                } else {
                    false
                }
            });
            if !overlaps {
                final_tables.push(text_table);
            }
        }
    }
    final_tables
}

fn grid_to_extracted_table(
    grid: &GridStructure,
    spans: &[TextSpan],
    visual_merge_info: Option<Vec<Vec<CellMergeInfo>>>,
) -> ExtractedTable {
    let num_rows = grid.cells.len();
    let num_cols = grid.columns.len();
    let merge_info = visual_merge_info.unwrap_or_else(|| detect_merged_cells(grid, spans));
    let header_row_idx = detect_header_row(grid, spans);
    let mut table_rows = Vec::new();
    for (row_idx, row) in grid.cells.iter().enumerate() {
        let is_header = header_row_idx == Some(row_idx);
        let mut table_row = TableRow::new(is_header);
        for (col_idx, cell_span_indices) in row.iter().enumerate() {
            let mi = &merge_info[row_idx][col_idx];
            if mi.covered {
                continue;
            }
            let cell_text = extract_cell_text(cell_span_indices, spans);
            let mut cell_bbox = None;
            if !cell_span_indices.is_empty() {
                let mut b = spans[cell_span_indices[0]].bbox;
                for &idx in &cell_span_indices[1..] {
                    b = b.union(&spans[idx].bbox);
                }
                cell_bbox = Some(b);
            }
            let mcids = cell_span_indices
                .iter()
                .filter_map(|&idx| spans.get(idx).and_then(|s| s.mcid))
                .collect::<Vec<_>>();
            table_row.cells.push(TableCell {
                text: cell_text,
                colspan: mi.colspan.min((num_cols - col_idx) as u32),
                rowspan: mi.rowspan.min((num_rows - row_idx) as u32),
                mcids,
                bbox: cell_bbox,
                is_header,
            });
        }
        table_rows.push(table_row);
    }
    let all_span_indices: Vec<usize> = grid
        .cells
        .iter()
        .flat_map(|row| row.iter().flat_map(|cell| cell.iter().copied()))
        .collect();
    let mut bbox = None;
    if !all_span_indices.is_empty() {
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        for &idx in &all_span_indices {
            if let Some(s) = spans.get(idx) {
                min_x = min_x.min(s.bbox.x);
                min_y = min_y.min(s.bbox.y);
                max_x = max_x.max(s.bbox.x + s.bbox.width);
                max_y = max_y.max(s.bbox.y + s.bbox.height);
            }
        }
        bbox = Some(crate::geometry::Rect::new(min_x, min_y, max_x - min_x, max_y - min_y));
    }
    ExtractedTable {
        rows: table_rows,
        has_header: header_row_idx.is_some(),
        col_count: num_cols,
        bbox,
    }
}

fn extract_cell_text(cell_span_indices: &[usize], spans: &[TextSpan]) -> String {
    if cell_span_indices.is_empty() {
        return String::new();
    }
    let mut span_entries: Vec<(f32, &str)> = cell_span_indices
        .iter()
        .filter_map(|&idx| spans.get(idx).map(|s| (s.bbox.center().y, s.text.as_str())))
        .collect();
    if span_entries.is_empty() {
        return String::new();
    }
    if span_entries.len() == 1 {
        return span_entries[0].1.to_string();
    }
    span_entries.sort_by(|a, b| crate::utils::safe_float_cmp(b.0, a.0));
    let mut lines: Vec<Vec<&str>> = Vec::new();
    let mut current_line: Vec<&str> = vec![span_entries[0].1];
    let mut current_y = span_entries[0].0;
    for &(y, text) in &span_entries[1..] {
        if (current_y - y).abs() <= 2.0 {
            current_line.push(text);
        } else {
            lines.push(current_line);
            current_line = vec![text];
            current_y = y;
        }
    }
    lines.push(current_line);
    lines
        .iter()
        .map(|line| line.join(" "))
        .collect::<Vec<_>>()
        .join("\n")
}

fn detect_merged_cells(grid: &GridStructure, spans: &[TextSpan]) -> Vec<Vec<CellMergeInfo>> {
    let num_rows = grid.cells.len();
    let num_cols = grid.columns.len();
    let mut merge_info: Vec<Vec<CellMergeInfo>> = (0..num_rows)
        .map(|_| {
            (0..num_cols)
                .map(|_| CellMergeInfo {
                    colspan: 1,
                    rowspan: 1,
                    covered: false,
                })
                .collect()
        })
        .collect();
    for row_idx in 0..num_rows {
        for col_idx in 0..num_cols {
            if grid.cells[row_idx][col_idx].is_empty() {
                continue;
            }
            let cell_right = grid.cells[row_idx][col_idx]
                .iter()
                .filter_map(|&idx| spans.get(idx).map(|s| s.bbox.right()))
                .fold(f32::NEG_INFINITY, f32::max);
            if cell_right == f32::NEG_INFINITY {
                continue;
            }
            let mut extra_cols = 0u32;
            for next_col in (col_idx + 1)..num_cols {
                if !grid.cells[row_idx][next_col].is_empty() {
                    break;
                }
                if cell_right > grid.columns[next_col].x_center {
                    extra_cols += 1;
                } else {
                    break;
                }
            }
            if extra_cols > 0 {
                merge_info[row_idx][col_idx].colspan = 1 + extra_cols;
                for c in 1..=(extra_cols as usize) {
                    merge_info[row_idx][col_idx + c].covered = true;
                }
            }
        }
    }
    for col_idx in 0..num_cols {
        for row_idx in 0..num_rows {
            if grid.cells[row_idx][col_idx].is_empty() || merge_info[row_idx][col_idx].covered {
                continue;
            }
            let cell_bottom = grid.cells[row_idx][col_idx]
                .iter()
                .filter_map(|&idx| spans.get(idx).map(|s| s.bbox.bottom()))
                .fold(f32::INFINITY, f32::min);
            if cell_bottom == f32::INFINITY {
                continue;
            }
            let mut extra_rows = 0u32;
            for next_row in (row_idx + 1)..num_rows {
                if !grid.cells[next_row][col_idx].is_empty() {
                    break;
                }
                if cell_bottom < grid.rows[next_row].y_center {
                    extra_rows += 1;
                } else {
                    break;
                }
            }
            if extra_rows > 0 {
                merge_info[row_idx][col_idx].rowspan = 1 + extra_rows;
                for r in 1..=(extra_rows as usize) {
                    merge_info[row_idx + r][col_idx].covered = true;
                }
            }
        }
    }
    merge_info
}

fn detect_header_row(grid: &GridStructure, spans: &[TextSpan]) -> Option<usize> {
    if grid.cells.len() < 2 {
        return None;
    }
    let first_row_spans: Vec<&TextSpan> = grid.cells[0]
        .iter()
        .flat_map(|cell| cell.iter().filter_map(|&idx| spans.get(idx)))
        .collect();
    if first_row_spans.is_empty() {
        return None;
    }
    let data_row_spans: Vec<&TextSpan> = grid.cells[1..]
        .iter()
        .flat_map(|row| {
            row.iter()
                .flat_map(|cell| cell.iter().filter_map(|&idx| spans.get(idx)))
        })
        .collect();
    if data_row_spans.is_empty() {
        return None;
    }
    let first_row_bold_ratio = first_row_spans
        .iter()
        .filter(|s| s.font_weight.is_bold())
        .count() as f32
        / first_row_spans.len() as f32;
    let data_bold_ratio = data_row_spans
        .iter()
        .filter(|s| s.font_weight.is_bold())
        .count() as f32
        / data_row_spans.len() as f32;
    if first_row_bold_ratio > 0.5 && data_bold_ratio < 0.3 {
        return Some(0);
    }
    let first_row_avg_size: f32 =
        first_row_spans.iter().map(|s| s.font_size).sum::<f32>() / first_row_spans.len() as f32;
    let data_avg_size: f32 =
        data_row_spans.iter().map(|s| s.font_size).sum::<f32>() / data_row_spans.len() as f32;
    if first_row_avg_size > data_avg_size + 1.5 {
        return Some(0);
    }
    Some(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Rect;
    use crate::layout::text_block::{Color, FontWeight};

    #[test]
    fn test_line_clustering_multiple_tables() {
        let lines = vec![
            make_rect_path(10.0, 100.0, 50.0, 20.0),
            make_rect_path(10.0, 50.0, 50.0, 20.0), // Far away vertically
        ];

        let clusters = group_lines_into_clusters(&lines);
        assert_eq!(
            clusters.len(),
            2,
            "Should find 2 separate table regions with optimized clustering"
        );
    }

    #[test]
    fn test_line_clustering_horizontal_separation() {
        let lines = vec![
            make_rect_path(10.0, 100.0, 50.0, 20.0), // Table 1: x=10..60
            make_rect_path(80.0, 100.0, 50.0, 20.0), // Table 2: x=80..130 (20pt gap)
        ];

        let clusters = group_lines_into_clusters(&lines);
        assert_eq!(
            clusters.len(),
            2,
            "Should find 2 separate table regions even if nearby horizontally"
        );
    }

    fn create_test_span(text: &str, x: f32, y: f32, width: f32, height: f32) -> TextSpan {
        TextSpan {
            artifact_type: None,
            text: text.to_string(),
            bbox: Rect::new(x, y, width, height),
            font_name: "TestFont".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            is_italic: false,
            color: Color::black(),
            mcid: None,
            sequence: 0,
            split_boundary_before: false,
            offset_semantic: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 1.0,
            primary_detected: false,
        }
    }
    fn make_h_line(x: f32, y: f32, width: f32) -> crate::elements::PathContent {
        crate::elements::PathContent::line(x, y, x + width, y)
    }
    fn make_v_line(x: f32, y: f32, height: f32) -> crate::elements::PathContent {
        crate::elements::PathContent::line(x, y, x, y + height)
    }
    fn make_line_path(x1: f32, y1: f32, x2: f32, y2: f32) -> crate::elements::PathContent {
        crate::elements::PathContent::line(x1, y1, x2, y2)
    }
    fn make_rect_path(x: f32, y: f32, w: f32, h: f32) -> crate::elements::PathContent {
        crate::elements::PathContent::rect(x, y, w, h)
    }

    #[test]
    fn test_lines_strategy_no_lines_returns_empty() {
        let spans = vec![
            create_test_span("A", 10.0, 100.0, 10.0, 10.0),
            create_test_span("B", 50.0, 100.0, 10.0, 10.0),
            create_test_span("C", 10.0, 80.0, 10.0, 10.0),
            create_test_span("D", 50.0, 80.0, 10.0, 10.0),
        ];
        let config = TableDetectionConfig {
            horizontal_strategy: TableStrategy::Lines,
            vertical_strategy: TableStrategy::Lines,
            ..TableDetectionConfig::default()
        };
        assert!(detect_tables_with_lines(&spans, &[], &config).is_empty());
    }

    #[test]
    fn test_table_splitting_on_empty_row() {
        let spans = vec![
            create_test_span("T1-11", 20.0, 115.0, 10.0, 10.0),
            create_test_span("T1-12", 40.0, 115.0, 10.0, 10.0),
            create_test_span("T1-21", 20.0, 95.0, 10.0, 10.0),
            create_test_span("T1-22", 40.0, 95.0, 10.0, 10.0),
            create_test_span("T2-11", 20.0, 35.0, 10.0, 10.0),
            create_test_span("T2-12", 40.0, 35.0, 10.0, 10.0),
            create_test_span("T2-21", 20.0, 15.0, 10.0, 10.0),
            create_test_span("T2-22", 40.0, 15.0, 10.0, 10.0),
        ];
        let lines = vec![
            make_h_line(10.0, 130.0, 50.0),
            make_h_line(10.0, 110.0, 50.0),
            make_h_line(10.0, 90.0, 50.0),
            make_v_line(10.0, 90.0, 40.0),
            make_v_line(30.0, 90.0, 40.0),
            make_v_line(60.0, 90.0, 40.0),
            make_h_line(10.0, 50.0, 50.0),
            make_h_line(10.0, 30.0, 50.0),
            make_h_line(10.0, 10.0, 50.0),
            make_v_line(10.0, 10.0, 40.0),
            make_v_line(30.0, 10.0, 40.0),
            make_v_line(60.0, 10.0, 40.0),
            make_v_line(10.0, 50.0, 40.0),
        ];
        let config = TableDetectionConfig {
            horizontal_strategy: TableStrategy::Both,
            vertical_strategy: TableStrategy::Both,
            ..TableDetectionConfig::default()
        };
        assert_eq!(detect_tables_with_lines(&spans, &lines, &config).len(), 2);
    }

    #[test]
    fn test_hierarchical_header_with_visual_heuristic() {
        let spans = vec![
            create_test_span("H1", 10.0, 115.0, 35.0, 10.0),
            create_test_span("H2", 55.0, 115.0, 35.0, 10.0),
            create_test_span("Col 1", 10.0, 95.0, 35.0, 10.0),
            create_test_span("Col 2", 55.0, 95.0, 35.0, 10.0),
            create_test_span("Data 1", 10.0, 75.0, 35.0, 10.0),
            create_test_span("Data 2", 55.0, 75.0, 35.0, 10.0),
        ];
        let lines = vec![
            make_line_path(10.0, 130.0, 90.0, 130.0),
            make_line_path(10.0, 110.0, 90.0, 110.0),
            make_line_path(10.0, 90.0, 90.0, 90.0),
            make_v_line(10.0, 70.0, 60.0),
            make_v_line(50.0, 70.0, 20.0),
            make_v_line(90.0, 70.0, 60.0),
        ];
        let config = TableDetectionConfig::default();
        let tables = detect_tables_with_lines(&spans, &lines, &config);
        assert_eq!(tables.len(), 1);
        assert!(tables[0].rows[0].is_header);
        assert!(tables[0].rows[1].is_header);
    }
}
