# GUI Design Decisions for query-os

## ✅ GUI Layout Design - Approved

**Decision:** The GUI should follow a clean, intuitive layout with the following components:

1. **Main Window Layout:**
   - Query input area at the top (multiline text box for SQL queries)
   - Execute button prominently placed next to the query input
   - Results table in the center/main area, taking up most of the window
   - Status bar at the bottom showing query execution time
   - Template management section (dropdown for loading templates, save button)

2. **User Experience Principles Applied:**
   - **User-centricity:** Query textbox should be the first focus, results immediately visible
   - **Consistency:** Use standard GUI controls (buttons, text areas, tables)
   - **Hierarchy:** Query input and execute button most prominent, results clearly displayed
   - **Context:** Clear labels and intuitive placement of all elements
   - **User control:** Ability to modify and re-run queries easily
   - **Accessibility:** Keyboard shortcuts for common actions (Enter to execute)
   - **Usability:** Error messages displayed clearly, loading states during query execution

3. **Visual Design:**
   - Clean, modern interface matching system theme
   - Query textbox with syntax highlighting if possible
   - Results table with sortable columns and scrollable view
   - Clear distinction between different result types (files, processes)

4. **Interaction Flow:**
   - User types or pastes query → clicks Execute or presses Enter
   - Results display in table format with appropriate columns
   - Template dropdown allows quick selection of saved queries
   - Save button allows saving current query as template

**Rationale:** This design puts the user's needs first by making query execution simple and results immediately visible, following established patterns from database GUI tools while maintaining consistency with the CLI tool's functionality.

## ✅ Modern UI Design - Approved

**Decision:** The GUI should follow modern design standards inspired by Spotify, Uber, and Airbnb with the following enhancements:

### 1. **Visual Design Updates:**
   - **Dark Theme:** Clean dark background (#1a1a1a) for reduced eye strain and modern aesthetics
   - **Typography:** Clear hierarchy with appropriate font sizes (28px title, 16px body, 12px status)
   - **Spacing:** Generous padding (20-25px sections, 15px internal spacing) for breathing room
   - **Layout:** Well-organized sections with clear visual separation

### 2. **Loading State & Spinner:**
   - **Animated Spinner:** Braided-style Unicode spinner (⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏) that cycles every 150ms
   - **Loading Overlay:** Spinner appears in results area during query execution
   - **Visual Feedback:** Button shows "Executing..." text during loading state
   - **Non-blocking UI:** Interface remains responsive during query execution

### 3. **Query Result Filtering:**
   - **Column Selection:** Only display columns explicitly selected in SELECT clause
   - **Dynamic Headers:** Column headers generated from selected field names with proper capitalization
   - **Field Mapping:** Correct mapping between SQL field names and data structure fields
   - **Type Handling:** Support for both filesystem (name, type, size, etc.) and process (pid, name, cpu, etc.) queries

### 4. **User Experience Improvements:**
   - **Empty State:** Clear messaging when no results are available
   - **Status Feedback:** Real-time status updates in bottom status bar
   - **Template Management:** Dropdown for saved query templates
   - **Intuitive Controls:** Prominent execute button, secondary save functionality

### 5. **Technical Implementation:**
   - **Iced Framework:** Built-in dark theme for consistent styling
   - **Subscription-based Animation:** Timer subscription for spinner animation during loading
   - **Message-driven Updates:** Clean separation between UI state and business logic
   - **Performance:** Efficient rendering with minimal re-draws

**Rationale:** This modern redesign addresses the original task requirements while following contemporary UI/UX principles. The dark theme provides a professional appearance, the animated spinner gives clear feedback during operations, and the column filtering ensures users see exactly what they requested. The design maintains usability while introducing modern visual elements that feel familiar from popular applications.

## ✅ Task 18 Features - Keyboard Shortcuts & Context Menu - Approved

**Decision:** Implement keyboard shortcuts and right-click context menu for enhanced user experience:

### 1. **Keyboard Shortcuts:**
   - **Cmd+Enter (Mac) / Ctrl+Enter (Windows/Linux):** Execute current query
   - **Cross-platform Support:** Automatically detects OS and uses appropriate modifier key
   - **Visual Feedback:** Warning message displayed when executing DELETE queries via shortcut
   - **Safety Feature:** Only works when not already loading/executing a query

### 2. **Right-Click Context Menu for Process Management:**
   - **Process Row Interaction:** Right-clicking any process row triggers kill action
   - **Automatic Query Generation:** Creates and executes `DELETE FROM ps WHERE pid = 'X'` query
   - **Safety First:** Confirmation dialog required before process termination
   - **Visual Design:** Modal overlay with clear warning and action buttons

### 3. **Confirmation Dialog Design:**
   - **Modal Overlay:** Semi-transparent dark overlay covers entire interface
   - **Dialog Container:** White rounded container with shadow and border
   - **Content Layout:** Clear title, descriptive text, warning in red, action buttons
   - **Button Styling:** Cancel (secondary style), Kill Process (destructive style)
   - **Typography:** Hierarchical text sizing (20px title, 16px description, 14px warning)

### 4. **User Experience Principles Applied:**
   - **User-centricity:** Keyboard shortcuts for power users, confirmation dialogs prevent accidents
   - **Consistency:** Follows existing GUI patterns and modern design standards
   - **Hierarchy:** Clear visual priority with prominent warnings and action buttons
   - **Context:** Right-click provides contextual actions directly on data rows
   - **User control:** Confirmation dialog gives users full control over destructive actions
   - **Accessibility:** Keyboard navigation works alongside mouse interactions
   - **Usability:** Intuitive right-click behavior, clear confirmation messaging

### 5. **Technical Implementation:**
   - **Event Handling:** Iced event subscription for keyboard events, mouse area for right-click
   - **State Management:** Pending kill PID tracking prevents race conditions
   - **Modal System:** Overlay rendering without disrupting main interface flow
   - **Cross-platform:** Conditional compilation for OS-specific keyboard shortcuts
   - **Error Handling:** Graceful failure with user feedback if process killing fails

### 6. **Safety Features:**
   - **Confirmation Required:** No process can be killed without explicit user confirmation
   - **Clear Warnings:** Red text emphasizes destructive nature of the action
   - **Audit Trail:** Status messages log all process termination attempts
   - **PID Validation:** Leverages existing query parsing for input validation

**Rationale:** These features significantly improve user productivity while maintaining safety. Keyboard shortcuts cater to power users who want to work efficiently, while the right-click context menu provides intuitive access to process management. The confirmation dialog prevents accidental system disruption, and the visual feedback ensures users are aware of potentially dangerous actions. This design balances functionality with safety, following modern GUI conventions while respecting the powerful nature of the underlying filesystem SQL operations.

## ✅ Extension Column Feature - Approved

**Decision:** Add an extension column to file queries with the following user experience considerations:

### 1. **Extension Column Behavior:**
   - **Column Name:** "extension" (lowercase, matches SQL field naming convention)
   - **Display Value:** File extension in lowercase (e.g., "txt", "rs", "pdf") or "NULL" for files/directories without extensions
   - **Data Type:** String value that can be NULL
   - **Sorting:** Case-insensitive alphabetical sorting (NULL values sort last)
   - **Filtering:** Supports all standard SQL operators (=, !=, LIKE, etc.) with case-insensitive matching

### 2. **User Experience Principles Applied:**
   - **Discoverability:** Extension column is available in all file queries alongside existing columns (name, type, size, etc.)
   - **Consistency:** Follows existing column behavior for display, sorting, and filtering
   - **Intuitiveness:** Users can immediately understand what the extension column represents
   - **User Control:** Extension can be selected, filtered, and sorted just like any other column
   - **Accessibility:** Column name is descriptive and follows SQL naming conventions
   - **Safety:** No additional security risks introduced, maintains existing query capabilities

### 3. **Query Examples and Use Cases:**
   - **Basic Selection:** `SELECT name, extension FROM .` - Shows files with their extensions
   - **Extension Filtering:** `SELECT name FROM . WHERE extension = 'rs'` - Find all Rust files
   - **Extension Sorting:** `SELECT name, extension FROM . ORDER BY extension` - Sort files by extension
   - **Pattern Matching:** `SELECT name FROM . WHERE extension LIKE 'j%'` - Find JavaScript/TypeScript files
   - **NULL Handling:** `SELECT name FROM . WHERE extension = 'NULL'` - Find files without extensions

### 4. **Edge Case Handling:**
   - **Directories:** Always display "NULL" (directories don't have extensions)
   - **Files without extensions:** Display "NULL" (e.g., "README", "Makefile")
   - **Hidden files starting with dot:** "NULL" if no extension after the dot (e.g., ".gitignore" → "NULL")
   - **Files with multiple dots:** Only the last part after the final dot is considered the extension (e.g., "archive.tar.gz" → "gz")
   - **Case sensitivity:** All extensions stored and compared in lowercase for consistency

### 5. **Technical Implementation:**
   - **Extension Extraction:** Safe string parsing with bounds checking to prevent panics
   - **Memory Efficiency:** Extension stored as Option<String> to minimize memory usage
   - **Performance:** Extension extracted once during file scanning, reused for all operations
   - **Thread Safety:** Extension extraction is pure function with no side effects

### 6. **User Experience Improvements:**
   - **Query Flexibility:** Users can now filter and sort by file type using familiar SQL syntax
   - **Data Insights:** Extension column provides additional metadata for filesystem analysis
   - **Workflow Efficiency:** Enables targeted queries like finding all source files, documents, etc.
   - **Consistency:** Maintains parity with CLI functionality in GUI interface

**Rationale:** The extension column adds valuable functionality that users would naturally expect from a filesystem query tool. By following existing patterns for column handling (display, sorting, filtering), the feature feels like a natural extension of the current capabilities. The decision to use "NULL" for missing extensions is intuitive and consistent with SQL semantics. This feature enables more sophisticated filesystem queries while maintaining the tool's simplicity and performance characteristics.
