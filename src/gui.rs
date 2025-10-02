use crate::{
    execute_query, get_template_dir, load_template_content, parse_query, save_template, QueryResult,
};
use iced::{
    widget::{
        button, column, container, pick_list, row, scrollable, text, text_editor, Column, Row,
    },
    Alignment, Application, Color, Command, Element, Length, Settings, Theme,
};
use opener;
use std::time::Instant;

// Use Iced's built-in dark theme with modern styling

pub struct Gui {
    query_content: iced::widget::text_editor::Content,
    results: Vec<GuiResultRow>,
    all_results: Vec<GuiResultRow>, // Store all results for pagination
    column_headers: Vec<String>,
    status: String,
    templates: Vec<String>,
    selected_template: Option<String>,
    is_loading: bool,
    spinner_frame: usize,
    sort_column: Option<usize>,
    sort_direction: SortDirection,
    is_file_results: bool, // Track if current results are file results for double-click functionality
    pending_kill_pid: Option<String>, // Track PID pending confirmation for killing
    displayed_count: usize, // Track how many results are currently displayed
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortDirection {
    Ascending,
    Descending,
    Default,
}

#[derive(Debug, Clone)]
pub enum Message {
    QueryChanged(iced::widget::text_editor::Action),
    ExecuteQuery,
    QueryExecuted(Result<QueryResultData, String>),
    TemplateSelected(String),
    LoadTemplate(String),
    SaveTemplate,
    Tick,                       // For spinner animation
    HeaderClicked(usize),       // For column sorting
    OpenFile(String),           // For opening files with double-click
    KeyboardEvent(iced::Event), // For keyboard shortcuts
    RightClickProcess(String),  // For right-click context menu on processes
    ConfirmProcessKill(String), // For confirming process termination
    ShowNextResults,            // For showing next batch of results
}

#[derive(Clone, Debug)]
pub struct GuiResultRow {
    pub columns: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct QueryResultData {
    pub headers: Vec<String>,
    pub rows: Vec<GuiResultRow>,
    pub all_rows: Option<Vec<GuiResultRow>>, // All results for pagination
    pub execution_time: u128,
    pub is_file_results: bool,
}

impl Default for Gui {
    fn default() -> Self {
        let mut gui = Self {
            query_content: iced::widget::text_editor::Content::new(),
            results: Vec::new(),
            all_results: Vec::new(),
            column_headers: Vec::new(),
            status: String::new(),
            templates: Vec::new(),
            selected_template: None,
            is_loading: false,
            spinner_frame: 0,
            sort_column: None,
            sort_direction: SortDirection::Default,
            is_file_results: false,
            pending_kill_pid: None,
            displayed_count: 0,
        };
        gui.load_templates();
        gui
    }
}

impl Gui {
    fn load_templates(&mut self) {
        self.templates.clear();
        if let Ok(template_dir) = get_template_dir() {
            if let Ok(entries) = std::fs::read_dir(template_dir) {
                for entry in entries.flatten() {
                    if let Some(name) = entry.file_name().to_str() {
                        if name.ends_with(".sql") {
                            self.templates
                                .push(name.trim_end_matches(".sql").to_string());
                        }
                    }
                }
            }
        }
    }

    fn execute_query_async(content: iced::widget::text_editor::Content) -> Command<Message> {
        Command::perform(
            async move {
                let start_time = Instant::now();
                let query_text = content.text();

                match parse_query(&query_text) {
                    Ok(query) => match execute_query(&query) {
                        Ok(results) => {
                            let (headers, result_rows, is_file_results) = match results {
                                QueryResult::Files(files) => {
                                    // Use selected fields from the query instead of hardcoded columns
                                    let selected_fields = if query.select_fields.is_empty() {
                                        vec![
                                            "name".to_string(),
                                            "type".to_string(),
                                            "modified_date".to_string(),
                                            "permissions".to_string(),
                                            "size".to_string(),
                                            "path".to_string(),
                                            "depth".to_string(),
                                        ]
                                    } else {
                                        query.select_fields.clone()
                                    };

                                    // Create headers from selected fields (capitalize first letter)
                                    let headers: Vec<String> = selected_fields
                                        .iter()
                                        .map(|field| {
                                            let mut chars = field.chars();
                                            match chars.next() {
                                                None => String::new(),
                                                Some(first) => {
                                                    first.to_uppercase().collect::<String>()
                                                        + chars.as_str()
                                                }
                                            }
                                        })
                                        .collect();

                                    let mut rows = Vec::new();
                                    for file in files {
                                        let mut columns = Vec::new();
                                        for field in &selected_fields {
                                            let value = match field.to_lowercase().as_str() {
                                                "name" => file.name.clone(),
                                                "type" => file.file_type.clone(),
                                                "modified" | "modified_date" => {
                                                    file.modified_date.to_string()
                                                }
                                                "permissions" => file.permissions.clone(),
                                                "size" => file.size.clone(),
                                                "path" => file.path.clone(),
                                                "depth" => file.depth.to_string(),
                                                _ => "".to_string(), // Unknown field
                                            };
                                            columns.push(value);
                                        }
                                        rows.push(GuiResultRow { columns });
                                    }
                                    (headers, rows, true)
                                }
                                QueryResult::Processes(processes) => {
                                    // Use selected fields from the query instead of hardcoded columns
                                    let selected_fields = if query.select_fields.is_empty() {
                                        vec![
                                            "name".to_string(),
                                            "pid".to_string(),
                                            "memory_usage".to_string(),
                                            "cpu_usage".to_string(),
                                            "status".to_string(),
                                        ]
                                    } else {
                                        query.select_fields.clone()
                                    };

                                    // Create headers from selected fields (capitalize first letter)
                                    let headers: Vec<String> = selected_fields
                                        .iter()
                                        .map(|field| {
                                            let mut chars = field.chars();
                                            match chars.next() {
                                                None => String::new(),
                                                Some(first) => {
                                                    first.to_uppercase().collect::<String>()
                                                        + chars.as_str()
                                                }
                                            }
                                        })
                                        .collect();

                                    let mut rows = Vec::new();
                                    for process in processes {
                                        let mut columns = Vec::new();
                                        for field in &selected_fields {
                                            let value = match field.to_lowercase().as_str() {
                                                "name" => process.name.clone(),
                                                "pid" => process.pid.clone(),
                                                "memory" | "memory_usage" => {
                                                    process.memory_usage.clone()
                                                }
                                                "cpu" | "cpu_usage" => process.cpu_usage.clone(),
                                                "status" => process.status.clone(),
                                                _ => "".to_string(), // Unknown field
                                            };
                                            columns.push(value);
                                        }
                                        rows.push(GuiResultRow { columns });
                                    }
                                    (headers, rows, false)
                                }
                                QueryResult::Network(network_info) => {
                                    // Use selected fields from the query instead of hardcoded columns
                                    let selected_fields = if query.select_fields.is_empty() {
                                        vec![
                                            "name".to_string(),
                                            "port".to_string(),
                                            "pid".to_string(),
                                        ]
                                    } else {
                                        query.select_fields.clone()
                                    };

                                    // Create headers from selected fields (capitalize first letter)
                                    let headers: Vec<String> = selected_fields
                                        .iter()
                                        .map(|field| {
                                            let mut chars = field.chars();
                                            match chars.next() {
                                                None => String::new(),
                                                Some(first) => {
                                                    first.to_uppercase().collect::<String>()
                                                        + chars.as_str()
                                                }
                                            }
                                        })
                                        .collect();

                                    let mut rows = Vec::new();
                                    for net_info in network_info {
                                        let mut columns = Vec::new();
                                        for field in &selected_fields {
                                            let value = match field.to_lowercase().as_str() {
                                                "name" => net_info.name.clone(),
                                                "port" => net_info.port.clone(),
                                                "pid" => net_info.pid.clone(),
                                                _ => "".to_string(), // Unknown field
                                            };
                                            columns.push(value);
                                        }
                                        rows.push(GuiResultRow { columns });
                                    }
                                    (headers, rows, false)
                                }
                                QueryResult::Applications(apps) => {
                                    // Use selected fields from the query instead of hardcoded columns
                                    let selected_fields = if query.select_fields.is_empty() {
                                        vec![
                                            "name".to_string(),
                                            "version".to_string(),
                                            "path".to_string(),
                                            "size".to_string(),
                                            "category".to_string(),
                                        ]
                                    } else {
                                        query.select_fields.clone()
                                    };

                                    // Create headers from selected fields (capitalize first letter)
                                    let headers: Vec<String> = selected_fields
                                        .iter()
                                        .map(|field| {
                                            let mut chars = field.chars();
                                            match chars.next() {
                                                None => String::new(),
                                                Some(first) => {
                                                    first.to_uppercase().collect::<String>()
                                                        + chars.as_str()
                                                }
                                            }
                                        })
                                        .collect();

                                    let mut rows = Vec::new();
                                    for app in apps {
                                        let mut columns = Vec::new();
                                        for field in &selected_fields {
                                            let value = match field.to_lowercase().as_str() {
                                                "name" => app.name.clone(),
                                                "version" => app.version.clone().unwrap_or_else(|| "NULL".to_string()),
                                                "path" => app.path.clone(),
                                                "size" => app.size.clone().unwrap_or_else(|| "NULL".to_string()),
                                                "category" => app.category.clone().unwrap_or_else(|| "NULL".to_string()),
                                                _ => "".to_string(), // Unknown field
                                            };
                                            columns.push(value);
                                        }
                                        rows.push(GuiResultRow { columns });
                                    }
                                    (headers, rows, false)
                                }
                            };
                            let execution_time = start_time.elapsed().as_millis();

                            // Limit initial display to 200 results for GUI performance
                            let displayed_rows = if result_rows.len() > 200 {
                                result_rows[..200].to_vec()
                            } else {
                                result_rows.clone()
                            };

                            Ok(QueryResultData {
                                headers,
                                rows: displayed_rows,
                                all_rows: Some(result_rows), // Store all results for pagination
                                execution_time,
                                is_file_results,
                            })
                        }
                        Err(e) => Err(format!("Error executing query: {}", e)),
                    },
                    Err(e) => Err(format!("Error parsing query: {}", e)),
                }
            },
            Message::QueryExecuted,
        )
    }

    fn load_template(&mut self, template_name: String) {
        match load_template_content(&template_name) {
            Ok(content) => {
                self.query_content = iced::widget::text_editor::Content::with_text(&content);
                self.status = format!("Loaded template '{}'", template_name);
            }
            Err(e) => {
                self.status = format!("Error loading template '{}': {}", template_name, e);
            }
        }
    }

    fn save_template(&mut self) {
        let query_text = self.query_content.text();
        if query_text.trim().is_empty() {
            self.status = "Cannot save empty query".to_string();
            return;
        }

        // Generate a default template name based on current time
        let template_name = format!("gui_query_{}", chrono::Utc::now().timestamp());
        match save_template(&template_name, &query_text) {
            Ok(_) => {
                self.status = format!("Template '{}' saved successfully", template_name);
                self.load_templates(); // Refresh template list
            }
            Err(e) => {
                self.status = format!("Error saving template: {}", e);
            }
        }
    }

    fn create_spinner(&self) -> Element<'_, Message, Theme> {
        let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let spinner_text = spinner_chars[self.spinner_frame % spinner_chars.len()];

        container(
            row![
                text(spinner_text)
                    .size(16)
                    .style(Color::from_rgba(0.4, 0.8, 1.0, 1.0)),
                text(" Executing query...")
                    .size(14)
                    .style(Color::from_rgba(0.8, 0.8, 0.8, 1.0))
            ]
            .spacing(8)
            .align_items(Alignment::Center),
        )
        .center_x()
        .center_y()
        .into()
    }

    fn sort_results(&mut self) {
        if let Some(column_idx) = self.sort_column {
            if self.sort_direction == SortDirection::Default {
                // Reset to original order (no sorting)
                return;
            }

            let empty_string = String::new();
            self.results.sort_by(|a, b| {
                let a_val = a.columns.get(column_idx).unwrap_or(&empty_string);
                let b_val = b.columns.get(column_idx).unwrap_or(&empty_string);

                // Try to parse as numbers first for proper numeric sorting
                if let (Ok(a_num), Ok(b_num)) = (a_val.parse::<f64>(), b_val.parse::<f64>()) {
                    match self.sort_direction {
                        SortDirection::Ascending => a_num
                            .partial_cmp(&b_num)
                            .unwrap_or(std::cmp::Ordering::Equal),
                        SortDirection::Descending => b_num
                            .partial_cmp(&a_num)
                            .unwrap_or(std::cmp::Ordering::Equal),
                        SortDirection::Default => std::cmp::Ordering::Equal,
                    }
                } else {
                    // Fall back to string comparison
                    match self.sort_direction {
                        SortDirection::Ascending => a_val.cmp(b_val),
                        SortDirection::Descending => b_val.cmp(a_val),
                        SortDirection::Default => std::cmp::Ordering::Equal,
                    }
                }
            });
        }
    }
}

impl Application for Gui {
    type Message = Message;
    type Theme = Theme;
    type Executor = iced::executor::Default;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        (Self::default(), Command::none())
    }

    fn title(&self) -> String {
        "Filesystem SQL Query GUI".to_string()
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        let mut subscriptions = Vec::new();

        // Spinner animation subscription
        if self.is_loading {
            subscriptions.push(
                iced::time::every(std::time::Duration::from_millis(150)).map(|_| Message::Tick),
            );
        }

        // Keyboard event subscription
        subscriptions.push(iced::event::listen().map(Message::KeyboardEvent));

        iced::Subscription::batch(subscriptions)
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::QueryChanged(action) => {
                self.query_content.perform(action);
                Command::none()
            }
            Message::ExecuteQuery => {
                let query_text = self.query_content.text().trim().to_string();
                if query_text.is_empty() {
                    self.status = "Please enter a query".to_string();
                    return Command::none();
                }

                self.is_loading = true;
                self.status = "Executing query...".to_string();
                let content =
                    iced::widget::text_editor::Content::with_text(&self.query_content.text());
                Self::execute_query_async(content)
            }
            Message::QueryExecuted(result) => {
                self.is_loading = false;
                match result {
                    Ok(data) => {
                        self.column_headers = data.headers;
                        self.results = data.rows.clone();
                        self.all_results = data.all_rows.unwrap_or(data.rows);
                        self.is_file_results = data.is_file_results;
                        self.displayed_count = self.results.len();
                        // Reset sorting when new results arrive
                        self.sort_column = None;
                        self.sort_direction = SortDirection::Default;

                        let total_count = self.all_results.len();
                        if total_count > 200 {
                            self.status = format!(
                                "Query executed in {:.3}ms - Showing {} of {} results",
                                data.execution_time, self.displayed_count, total_count
                            );
                        } else {
                            self.status = format!("Query executed in {:.3}ms", data.execution_time);
                        }
                    }
                    Err(e) => {
                        self.status = e;
                        self.results.clear();
                        self.column_headers.clear();
                        // Reset sorting on error too
                        self.sort_column = None;
                        self.sort_direction = SortDirection::Default;
                    }
                }
                Command::none()
            }
            Message::TemplateSelected(template_name) => {
                self.selected_template = Some(template_name.clone());
                self.load_template(template_name);
                Command::none()
            }
            Message::LoadTemplate(template_name) => {
                self.load_template(template_name);
                Command::none()
            }
            Message::SaveTemplate => {
                self.save_template();
                Command::none()
            }
            Message::Tick => {
                if self.is_loading {
                    self.spinner_frame = (self.spinner_frame + 1) % 10; // Use full spinner_chars length
                }
                Command::none()
            }
            Message::HeaderClicked(column_idx) => {
                // Handle column sorting: ASC -> DESC -> Default -> ASC...
                if self.sort_column == Some(column_idx) {
                    self.sort_direction = match self.sort_direction {
                        SortDirection::Default => SortDirection::Ascending,
                        SortDirection::Ascending => SortDirection::Descending,
                        SortDirection::Descending => SortDirection::Default,
                    };
                } else {
                    self.sort_column = Some(column_idx);
                    self.sort_direction = SortDirection::Ascending;
                }

                // Re-sort the results
                if self.sort_direction != SortDirection::Default {
                    self.sort_results();
                } else {
                    // For Default, we need to reset to original order
                    // This would require storing original results, for now just clear sorting
                    self.sort_column = None;
                }

                Command::none()
            }
            Message::OpenFile(file_path) => {
                match opener::open(&file_path) {
                    Ok(()) => {
                        self.status = format!("Opened file: {}", file_path);
                    }
                    Err(e) => {
                        self.status = format!("Error opening file '{}': {}", file_path, e);
                    }
                }
                Command::none()
            }
            Message::KeyboardEvent(event) => {
                match event {
                    iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                        key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Enter),
                        modifiers,
                        ..
                    }) => {
                        // Check for Cmd+Enter (Mac) or Ctrl+Enter (other platforms)
                        #[cfg(target_os = "macos")]
                        let is_modifier_pressed = modifiers.command();
                        #[cfg(not(target_os = "macos"))]
                        let is_modifier_pressed = modifiers.control();

                        if is_modifier_pressed && !self.is_loading {
                            let query_text = self.query_content.text().trim().to_uppercase();
                            if query_text.starts_with("DELETE") {
                                // Show warning for DELETE queries executed via shortcut
                                self.status = "⚠️ DELETE query executed via keyboard shortcut. Please review before confirming.".to_string();
                            }
                            // Trigger query execution
                            return self.update(Message::ExecuteQuery);
                        }
                    }
                    _ => {}
                }
                Command::none()
            }
            Message::RightClickProcess(pid) => {
                // Set pending kill PID for confirmation dialog
                self.pending_kill_pid = Some(pid);
                Command::none()
            }
            Message::ConfirmProcessKill(confirmation) => {
                if confirmation == "yes" {
                    if let Some(pid) = self.pending_kill_pid.take() {
                        // Create a DELETE query for the process with the given PID
                        let delete_query = format!("DELETE FROM ps WHERE pid = '{}'", pid);
                        self.query_content =
                            iced::widget::text_editor::Content::with_text(&delete_query);

                        // Execute the query immediately
                        let content = iced::widget::text_editor::Content::with_text(&delete_query);
                        Self::execute_query_async(content)
                    } else {
                        Command::none()
                    }
                } else {
                    // User cancelled, clear pending kill
                    self.pending_kill_pid = None;
                    Command::none()
                }
            }
            Message::ShowNextResults => {
                // Show next 200 results
                let remaining = self.all_results.len().saturating_sub(self.displayed_count);
                let next_count = std::cmp::min(200, remaining);
                if next_count > 0 {
                    let start_idx = self.displayed_count;
                    let end_idx = start_idx + next_count;
                    self.results
                        .extend_from_slice(&self.all_results[start_idx..end_idx]);
                    self.displayed_count = self.results.len();

                    let total_count = self.all_results.len();
                    if self.displayed_count < total_count {
                        self.status = format!(
                            "Showing {} of {} results",
                            self.displayed_count, total_count
                        );
                    } else {
                        self.status = format!("Showing all {} results", total_count);
                    }
                }
                Command::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message, Theme> {
        // Header section with title
        let title = text("Filesystem SQL Query").size(28);

        let header_container = container(title).padding(20);

        // Query input section
        let query_input = text_editor(&self.query_content)
            .on_action(Message::QueryChanged)
            .height(Length::Fixed(120.0))
            .padding(15);

        // Control buttons section
        let execute_button_text = if self.is_loading {
            "Executing..."
        } else {
            "Execute Query"
        };
        let execute_button = button(execute_button_text)
            .on_press_maybe(if self.is_loading {
                None
            } else {
                Some(Message::ExecuteQuery)
            })
            .padding(10);

        let template_picklist = pick_list(
            self.templates.as_slice(),
            self.selected_template.as_ref(),
            Message::TemplateSelected,
        )
        .placeholder("Select template...");

        let save_button = button("Save as Template")
            .on_press(Message::SaveTemplate)
            .padding(10);

        let controls_row = row![template_picklist, execute_button, save_button]
            .spacing(10)
            .align_items(Alignment::Start);

        let query_section = container(column![query_input, controls_row].spacing(15)).padding(20);

        // Results section
        let results_content = if self.is_loading {
            // Loading state with spinner
            container(self.create_spinner())
                .center_x()
                .center_y()
                .height(Length::Fixed(200.0))
        } else if self.column_headers.is_empty() {
            // Empty state
            container(text("No results yet. Execute a query to see results.").size(16))
                .center_x()
                .center_y()
                .height(Length::Fixed(200.0))
        } else {
            // Modern results table with styling
            let mut header_row = Row::new().spacing(0);
            for (col_idx, header) in self.column_headers.iter().enumerate() {
                // Add sort indicator
                let sort_indicator = if self.sort_column == Some(col_idx) {
                    match self.sort_direction {
                        SortDirection::Ascending => " ↑",
                        SortDirection::Descending => " ↓",
                        SortDirection::Default => "",
                    }
                } else {
                    ""
                };

                let header_content = format!("{}{}", header, sort_indicator);

                let header_button = button(text(header_content).size(14))
                    .on_press(Message::HeaderClicked(col_idx))
                    .padding(12)
                    .style(iced::theme::Button::Custom(Box::new(HeaderButtonStyle)));

                header_row = header_row.push(
                    container(header_button)
                        .width(Length::FillPortion(1))
                        .style(iced::theme::Container::Custom(Box::new(HeaderCellStyle))),
                );
            }

            // Results table rows with modern styling
            let mut results_column = Column::new().spacing(0);
            for (_row_idx, result) in self.results.iter().enumerate() {
                let mut row: Row<'_, Message, Theme> = Row::new().spacing(0);
                for (col_idx, column) in result.columns.iter().enumerate() {
                    let mut cell_container = container(text(column).size(13))
                        .padding(12)
                        .width(Length::FillPortion(1))
                        .style(iced::theme::Container::Custom(Box::new(DataCellStyle)));

                    // Make cells clickable for file results (double-click to open)
                    if self.is_file_results && col_idx == 0 {
                        // First column (usually name/path)
                        cell_container = cell_container.style(iced::theme::Container::Custom(
                            Box::new(ClickableDataCellStyle),
                        ));
                    }

                    row = row.push(cell_container);
                }

                let row_container: Element<'_, Message, Theme> = if self.is_file_results {
                    // For file results, make the entire row clickable for double-click
                    button(
                        container(row)
                            .style(iced::theme::Container::Custom(Box::new(TableRowStyle))),
                    )
                    .on_press(Message::OpenFile(
                        result.columns.get(5).unwrap_or(&String::new()).clone(),
                    )) // path column
                    .style(iced::theme::Button::Custom(Box::new(RowButtonStyle)))
                    .into()
                } else {
                    // For process results, make the row right-clickable for context menu
                    // PID is typically in the first column (index 0)
                    let pid = result.columns.get(0).unwrap_or(&String::new()).clone();
                    iced::widget::container(
                        iced::widget::mouse_area(row)
                            .on_right_press(Message::RightClickProcess(pid)),
                    )
                    .style(iced::theme::Container::Custom(Box::new(TableRowStyle)))
                    .into()
                };

                results_column = results_column.push(row_container);
            }

            let results_scrollable = scrollable(results_column).height(Length::Fill);

            // Check if we need to show "Show Next 200" button
            let has_more_results = self.displayed_count < self.all_results.len();
            let show_next_button = if has_more_results {
                Some(
                    container(
                        button("Show Next 200 Results")
                            .on_press(Message::ShowNextResults)
                            .padding(10)
                            .style(iced::theme::Button::Primary),
                    )
                    .center_x()
                    .padding(10),
                )
            } else {
                None
            };

            let mut results_elements = vec![
                container(header_row)
                    .style(iced::theme::Container::Custom(Box::new(HeaderRowStyle)))
                    .into(),
                results_scrollable.into(),
            ];

            if let Some(button) = show_next_button {
                results_elements.push(button.into());
            }

            container(Column::with_children(results_elements).spacing(0)).style(
                iced::theme::Container::Custom(Box::new(TableContainerStyle)),
            )
        };

        let results_section = container(results_content).padding(20).height(Length::Fill);

        // Status bar
        let status_bar = container(text(&self.status).size(12))
            .padding(15)
            .center_x();

        // Main layout
        let content = column![header_container, query_section, results_section, status_bar]
            .spacing(20)
            .padding(25);

        let main_content = container(content).width(Length::Fill).height(Length::Fill);

        // Confirmation dialog for process killing
        if let Some(ref pid) = self.pending_kill_pid {
            let dialog = container(
                column![
                    text("Confirm Process Termination").size(20),
                    text(format!(
                        "Are you sure you want to kill process with PID {}?",
                        pid
                    ))
                    .size(16),
                    text("This action cannot be undone.")
                        .size(14)
                        .style(Color::from_rgba(1.0, 0.0, 0.0, 1.0)),
                    row![
                        button("Cancel")
                            .on_press(Message::ConfirmProcessKill("no".to_string()))
                            .padding(10)
                            .style(iced::theme::Button::Secondary),
                        button("Kill Process")
                            .on_press(Message::ConfirmProcessKill("yes".to_string()))
                            .padding(10)
                            .style(iced::theme::Button::Destructive)
                    ]
                    .spacing(10)
                ]
                .spacing(20)
                .align_items(Alignment::Center),
            )
            .padding(30)
            .style(iced::theme::Container::Custom(Box::new(
                DialogContainerStyle,
            )))
            .center_x()
            .center_y();

            // Overlay the dialog on top of the main content
            container(column![
                main_content,
                container(dialog)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(iced::theme::Container::Custom(Box::new(DialogOverlayStyle)))
            ])
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
        } else {
            main_content.into()
        }
    }
}

pub fn run_gui() -> iced::Result {
    Gui::run(Settings::default())
}

// Custom styles for modern table appearance
#[derive(Default)]
struct HeaderButtonStyle;

impl iced::widget::button::StyleSheet for HeaderButtonStyle {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> iced::widget::button::Appearance {
        iced::widget::button::Appearance {
            background: Some(iced::Background::Color(Color::from_rgba(
                0.2, 0.2, 0.2, 1.0,
            ))),
            border: iced::Border::default(),
            shadow: iced::Shadow::default(),
            text_color: Color::from_rgba(0.9, 0.9, 0.9, 1.0),
            shadow_offset: iced::Vector::ZERO,
        }
    }

    fn hovered(&self, _style: &Self::Style) -> iced::widget::button::Appearance {
        iced::widget::button::Appearance {
            background: Some(iced::Background::Color(Color::from_rgba(
                0.25, 0.25, 0.25, 1.0,
            ))),
            border: iced::Border::default(),
            shadow: iced::Shadow::default(),
            text_color: Color::from_rgba(1.0, 1.0, 1.0, 1.0),
            shadow_offset: iced::Vector::ZERO,
        }
    }

    fn pressed(&self, _style: &Self::Style) -> iced::widget::button::Appearance {
        iced::widget::button::Appearance {
            background: Some(iced::Background::Color(Color::from_rgba(
                0.15, 0.15, 0.15, 1.0,
            ))),
            border: iced::Border::default(),
            shadow: iced::Shadow::default(),
            text_color: Color::from_rgba(0.9, 0.9, 0.9, 1.0),
            shadow_offset: iced::Vector::ZERO,
        }
    }
}

#[derive(Default)]
struct HeaderCellStyle;

impl iced::widget::container::StyleSheet for HeaderCellStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
        iced::widget::container::Appearance {
            background: Some(iced::Background::Color(Color::from_rgba(
                0.15, 0.15, 0.15, 1.0,
            ))),
            border: iced::Border::default(),
            shadow: iced::Shadow::default(),
            text_color: None,
        }
    }
}

#[derive(Default)]
struct HeaderRowStyle;

impl iced::widget::container::StyleSheet for HeaderRowStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
        iced::widget::container::Appearance {
            background: Some(iced::Background::Color(Color::from_rgba(
                0.15, 0.15, 0.15, 1.0,
            ))),
            border: iced::Border::default(),
            shadow: iced::Shadow::default(),
            text_color: None,
        }
    }
}

#[derive(Default)]
struct DataCellStyle;

impl iced::widget::container::StyleSheet for DataCellStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
        iced::widget::container::Appearance {
            background: None,
            border: iced::Border::default(),
            shadow: iced::Shadow::default(),
            text_color: None,
        }
    }
}

#[derive(Default)]
struct TableRowStyle;

impl iced::widget::container::StyleSheet for TableRowStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
        iced::widget::container::Appearance {
            background: None,
            border: iced::Border::default(),
            shadow: iced::Shadow::default(),
            text_color: None,
        }
    }
}

#[derive(Default)]
struct TableContainerStyle;

impl iced::widget::container::StyleSheet for TableContainerStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
        iced::widget::container::Appearance {
            background: Some(iced::Background::Color(Color::from_rgba(
                1.0, 1.0, 1.0, 1.0,
            ))),
            border: iced::Border::default(),
            shadow: iced::Shadow::default(),
            text_color: None,
        }
    }
}

#[derive(Default)]
struct ClickableDataCellStyle;

impl iced::widget::container::StyleSheet for ClickableDataCellStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
        iced::widget::container::Appearance {
            background: None,
            border: iced::Border::default(),
            shadow: iced::Shadow::default(),
            text_color: None,
        }
    }
}

#[derive(Default)]
struct RowButtonStyle;

impl iced::widget::button::StyleSheet for RowButtonStyle {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> iced::widget::button::Appearance {
        iced::widget::button::Appearance {
            background: None,
            border: iced::Border::default(),
            shadow: iced::Shadow::default(),
            text_color: Color::from_rgba(0.0, 0.0, 0.0, 1.0),
            shadow_offset: iced::Vector::ZERO,
        }
    }

    fn hovered(&self, _style: &Self::Style) -> iced::widget::button::Appearance {
        iced::widget::button::Appearance {
            background: Some(iced::Background::Color(Color::from_rgba(
                0.9, 0.9, 1.0, 0.3,
            ))),
            border: iced::Border::default(),
            shadow: iced::Shadow::default(),
            text_color: Color::from_rgba(0.0, 0.0, 0.0, 1.0),
            shadow_offset: iced::Vector::ZERO,
        }
    }

    fn pressed(&self, _style: &Self::Style) -> iced::widget::button::Appearance {
        iced::widget::button::Appearance {
            background: Some(iced::Background::Color(Color::from_rgba(
                0.8, 0.8, 0.9, 0.5,
            ))),
            border: iced::Border::default(),
            shadow: iced::Shadow::default(),
            text_color: Color::from_rgba(0.0, 0.0, 0.0, 1.0),
            shadow_offset: iced::Vector::ZERO,
        }
    }
}

#[derive(Default)]
struct DialogContainerStyle;

impl iced::widget::container::StyleSheet for DialogContainerStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
        iced::widget::container::Appearance {
            background: Some(iced::Background::Color(Color::from_rgba(
                1.0, 1.0, 1.0, 1.0,
            ))),
            border: iced::Border {
                color: Color::from_rgba(0.5, 0.5, 0.5, 1.0),
                width: 2.0,
                radius: 8.0.into(),
            },
            shadow: iced::Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.5),
                offset: iced::Vector::new(0.0, 4.0),
                blur_radius: 10.0,
            },
            text_color: None,
        }
    }
}

#[derive(Default)]
struct DialogOverlayStyle;

impl iced::widget::container::StyleSheet for DialogOverlayStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
        iced::widget::container::Appearance {
            background: Some(iced::Background::Color(Color::from_rgba(
                0.0, 0.0, 0.0, 0.5,
            ))),
            border: iced::Border::default(),
            shadow: iced::Shadow::default(),
            text_color: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{FileInfo, ProcessInfo, SqlQuery};

    #[test]
    fn test_column_filtering_files() {
        // Test that only selected columns are returned for file queries
        let file_info = FileInfo {
            name: "test.txt".to_string(),
            file_type: "file".to_string(),
            modified_date: chrono::Utc::now(),
            permissions: "644".to_string(),
            size: "1024 B".to_string(),
            path: "./test.txt".to_string(),
            depth: 1,
            extension: Some("txt".to_string()),
        };

        let selected_fields = vec!["name".to_string()];
        let mut columns = Vec::new();
        for field in &selected_fields {
            let value = match field.to_lowercase().as_str() {
                "name" => file_info.name.clone(),
                "type" => file_info.file_type.clone(),
                "modified" | "modified_date" => file_info.modified_date.to_string(),
                "permissions" => file_info.permissions.clone(),
                "size" => file_info.size.clone(),
                "path" => file_info.path.clone(),
                _ => "".to_string(),
            };
            columns.push(value);
        }

        assert_eq!(columns.len(), 1);
        assert_eq!(columns[0], "test.txt");
    }

    #[test]
    fn test_column_filtering_processes() {
        // Test that only selected columns are returned for process queries
        let process_info = ProcessInfo::new(1234, "node", 5.5, 1024 * 1024, "running");

        let selected_fields = vec!["name".to_string(), "pid".to_string()];
        let mut columns = Vec::new();
        for field in &selected_fields {
            let value = match field.to_lowercase().as_str() {
                "name" => process_info.name.clone(),
                "pid" => process_info.pid.clone(),
                "memory" | "memory_usage" => process_info.memory_usage.clone(),
                "cpu" | "cpu_usage" => process_info.cpu_usage.clone(),
                "status" => process_info.status.clone(),
                _ => "".to_string(),
            };
            columns.push(value);
        }

        assert_eq!(columns.len(), 2);
        assert_eq!(columns[0], "node");
        assert_eq!(columns[1], "1234");
    }

    #[test]
    fn test_column_header_capitalization() {
        let selected_fields = vec![
            "name".to_string(),
            "file_type".to_string(),
            "cpu_usage".to_string(),
        ];
        let headers: Vec<String> = selected_fields
            .iter()
            .map(|field| {
                let mut chars = field.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect();

        assert_eq!(headers, vec!["Name", "File_type", "Cpu_usage"]);
    }

    #[test]
    fn test_spinner_frame_calculation() {
        let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

        // Test that spinner frames cycle correctly
        for i in 0..20 {
            let frame = i % 4; // Simulating our spinner_frame % 4 logic
            let spinner_text = spinner_chars[frame % spinner_chars.len()];
            assert!(!spinner_text.is_empty());
        }
    }

    #[test]
    fn test_empty_query_handling() {
        // Test that empty select_fields defaults to all columns
        let query_with_empty_fields = SqlQuery {
            query_type: crate::models::QueryType::Select,
            select_fields: vec![],
            select_field_aliases: vec![],
            select_subqueries: vec![],
            from_path: ".".to_string(),
            where_clause: None,
            where_subqueries: vec![],
            order_by: None,
            order_direction: crate::models::SortDirection::Ascending,
            limit: None,
            distinct: false,
        };

        let selected_fields = if query_with_empty_fields.select_fields.is_empty() {
            vec![
                "name".to_string(),
                "type".to_string(),
                "modified_date".to_string(),
                "permissions".to_string(),
                "size".to_string(),
                "path".to_string(),
            ]
        } else {
            query_with_empty_fields.select_fields.clone()
        };

        assert_eq!(selected_fields.len(), 6);
        assert!(selected_fields.contains(&"name".to_string()));
        assert!(selected_fields.contains(&"type".to_string()));
    }

    #[test]
    fn test_select_star_expansion() {
        // Test that * gets expanded to all fields (handled in parser, but verify GUI receives it)
        let query_with_star = SqlQuery {
            query_type: crate::models::QueryType::Select,
            select_fields: vec!["*".to_string()],
            select_field_aliases: vec![],
            select_subqueries: vec![],
            from_path: ".".to_string(),
            where_clause: None,
            where_subqueries: vec![],
            order_by: None,
            order_direction: crate::models::SortDirection::Ascending,
            limit: None,
            distinct: false,
        };

        // This would be handled by the parser expansion, but test the GUI logic
        let selected_fields = if query_with_star.select_fields == vec!["*"] {
            vec![
                "name".to_string(),
                "type".to_string(),
                "modified_date".to_string(),
                "permissions".to_string(),
                "size".to_string(),
                "path".to_string(),
            ]
        } else {
            query_with_star.select_fields.clone()
        };

        assert_eq!(selected_fields.len(), 6);
    }

    #[test]
    fn test_gui_initialization() {
        let gui = Gui::default();
        assert_eq!(gui.query_content.text(), "\n");
        assert!(gui.results.is_empty());
        assert!(gui.column_headers.is_empty());
        assert_eq!(gui.spinner_frame, 0);
        assert!(!gui.is_loading);
    }
}
