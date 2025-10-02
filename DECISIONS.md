# Product Decisions

## Process Querying Feature - Task 5

**Decision**: Include the following fields for process querying:
- PID (Process ID)
- Name (Process name/command)
- CPU usage (percentage)
- Memory usage (in KB/MB/GB format)
- Status (running, sleeping, etc.)

**Rationale**: These are the most commonly queried process attributes and match standard system monitoring tools. Memory usage will be formatted consistently with filesystem size formatting for user familiarity.

**Approved by**: Jason (Product Owner)
**Date**: September 22, 2025

## Case-Insensitive SQL Keywords - Task 8

**Decision**: ✅ Implement case-insensitive parsing for all SQL keywords including SELECT, FROM, WHERE, DELETE, AND, LIKE, NOT, ORDER, BY, LIMIT.

**Rationale**: Improves user experience by allowing flexible case usage in queries, making the tool more intuitive and less error-prone. All keywords now accept both uppercase and lowercase variations.

**Approved by**: Jason (Product Owner)
**Date**: September 22, 2025

## Subquery Support - Task 9

**Decision**: ✅ Implement subqueries in WHERE clauses and SELECT statements.

**Rationale**: Enables more complex queries such as finding files that are larger than the average file size, or selecting process information based on criteria from other queries. Supports both correlated and non-correlated subqueries in WHERE conditions (using IN, EXISTS, comparison operators) and scalar subqueries in SELECT clauses.

**Approved by**: Jason (Product Owner)
**Date**: September 22, 2025

## Web Scraping Feature - Task 10

**Decision**: ✅ Implement web scraping functionality with the following specifications:
- Support for URLs in FROM clause
- CSS selector support in SELECT statement
- Wildcard (*) support for raw HTML output
- Text extraction with ::text pseudo-selector
- Additional output formats (JSON, etc.)
- Proper error handling for network failures
- Loading animation for web queries

**Rationale**: Extends filesystem SQL capabilities to web content, enabling powerful data extraction from websites using familiar SQL syntax. CSS selectors provide precise content targeting while maintaining intuitive interface.

**Approved by**: Jason (Product Owner)
**Date**: September 22, 2025

## GUI Interface - Task 13

**Decision**: ✅ Implement GUI interface with --gui flag supporting:
- Query input textbox with SQL syntax
- Execute button for running queries
- Results display in table format
- Template loading via dropdown selector
- Template saving functionality
- Status messages for query execution feedback
- Full support for existing CLI functionality (templates, basic queries)

**Rationale**: Provides user-friendly graphical interface for filesystem SQL queries, making the tool accessible to users who prefer GUI applications. Maintains all existing functionality while adding visual query building and result viewing capabilities.

**Approved by**: Jason (Product Owner)
**Date**: September 23, 2025

## Task 18 GUI Enhancements - Keyboard Shortcuts & Context Menu

**Decision**: ✅ Implement keyboard shortcuts and right-click context menu for enhanced productivity and usability:

### Keyboard Shortcuts:
- **Cmd+Enter (Mac) / Ctrl+Enter (Windows/Linux)**: Execute current query instantly
- Cross-platform modifier key detection
- Visual warning feedback for DELETE queries executed via shortcuts
- Safety guard preventing execution during loading states

### Right-Click Context Menu:
- **Process Row Right-Click**: Direct access to process termination functionality
- **Automatic DELETE Query Generation**: Creates `DELETE FROM ps WHERE pid = 'X'` queries
- **Safety-First Design**: Mandatory confirmation dialog before process termination
- **Intuitive Interaction**: Right-click directly on process rows for immediate action

### Confirmation Dialog:
- **Modal Safety Overlay**: Prevents accidental interaction during confirmation
- **Clear Visual Hierarchy**: Title, description, red warning text, action buttons
- **Two-Button Choice**: Cancel (secondary) vs Kill Process (destructive styling)
- **Professional Appearance**: Rounded corners, shadows, proper spacing

**Rationale**: These enhancements dramatically improve user productivity while maintaining system safety. Keyboard shortcuts cater to power users who need rapid query execution, while the right-click context menu provides intuitive process management. The confirmation dialog prevents accidental system disruption, ensuring users have full control over destructive operations.

**Quality Standards Met**:
- **User-centricity**: Features designed for both novice and expert users with appropriate safety measures
- **Consistency**: Follows existing GUI patterns and modern design conventions
- **Hierarchy**: Clear visual priority with warnings and confirmation flows
- **Context**: Right-click provides contextual actions directly on relevant data
- **User control**: Full user control over destructive actions with clear confirmation
- **Accessibility**: Keyboard navigation complements mouse interactions
- **Usability**: Intuitive behaviors with comprehensive safety measures

**Technical Implementation**:
- Event-driven architecture using Iced's subscription system
- Cross-platform keyboard shortcut detection with conditional compilation
- Modal dialog system with proper state management
- Comprehensive error handling and user feedback
- Maintains all existing functionality while adding productivity features

**Security Considerations Addressed**:
- Confirmation required for all process termination actions
- Visual warnings for potentially destructive operations
- Input validation through existing query parsing infrastructure
- Clear audit trail through status message logging

**Approved by**: Jason (Product Owner)
**Date**: September 23, 2025

## GUI Modernization - Task 16

**Decision**: ✅ Implement GUI modernization improvements including:
- Modern table design with clean styling, borders, and visual hierarchy inspired by contemporary design patterns
- Clickable column headers with three-state sorting (ASC → DESC → Default)
- Fixed loading spinner animation using full 10-character spinner sequence
- Improved user experience with intuitive visual feedback and modern UI elements

**Rationale**: These improvements significantly enhance the user experience by providing:
- Professional, modern table appearance that feels familiar from popular applications
- Essential data exploration functionality through column sorting
- Proper visual feedback during query execution with working spinner animation
- Better usability and accessibility following UX best practices

**Quality Standards Met**:
- User-centricity: Table design puts data exploration first with intuitive sorting
- Consistency: Modern styling consistent with contemporary application design
- Hierarchy: Clear visual distinction between headers, data, and interactive elements
- Context: Appropriate visual feedback for different application states
- User control: Clickable headers give users control over data presentation
- Accessibility: Proper contrast, sizing, and interactive elements
- Usability: Intuitive interactions with clear visual states

**Technical Implementation**:
- Sorting supports both numeric and string data with proper ordering
- Spinner animation cycles through complete Unicode spinner sequence
- Modern styling uses custom Iced stylesheets for professional appearance
- Maintains all existing functionality while adding new capabilities
- Code follows Rust best practices with proper error handling

**Approved by**: Jason (Product Owner)
**Date**: September 23, 2025

## Task 18 GUI Enhancements - Keyboard Shortcuts & Context Menu

**Decision**: ✅ Implement keyboard shortcuts and right-click context menu for enhanced productivity and usability:

### Keyboard Shortcuts:
- **Cmd+Enter (Mac) / Ctrl+Enter (Windows/Linux)**: Execute current query instantly
- Cross-platform modifier key detection
- Visual warning feedback for DELETE queries executed via shortcuts
- Safety guard preventing execution during loading states

### Right-Click Context Menu:
- **Process Row Right-Click**: Direct access to process termination functionality
- **Automatic DELETE Query Generation**: Creates `DELETE FROM ps WHERE pid = 'X'` queries
- **Safety-First Design**: Mandatory confirmation dialog before process termination
- **Intuitive Interaction**: Right-click directly on process rows for immediate action

### Confirmation Dialog:
- **Modal Safety Overlay**: Prevents accidental interaction during confirmation
- **Clear Visual Hierarchy**: Title, description, red warning text, action buttons
- **Two-Button Choice**: Cancel (secondary) vs Kill Process (destructive styling)
- **Professional Appearance**: Rounded corners, shadows, proper spacing

**Rationale**: These enhancements dramatically improve user productivity while maintaining system safety. Keyboard shortcuts cater to power users who need rapid query execution, while the right-click context menu provides intuitive process management. The confirmation dialog prevents accidental system disruption, ensuring users have full control over destructive operations.

**Quality Standards Met**:
- **User-centricity**: Features designed for both novice and expert users with appropriate safety measures
- **Consistency**: Follows existing GUI patterns and modern design conventions
- **Hierarchy**: Clear visual priority with warnings and confirmation flows
- **Context**: Right-click provides contextual actions directly on relevant data
- **User control**: Full user control over destructive actions with clear confirmation
- **Accessibility**: Keyboard navigation complements mouse interactions
- **Usability**: Intuitive behaviors with comprehensive safety measures

**Technical Implementation**:
- Event-driven architecture using Iced's subscription system
- Cross-platform keyboard shortcut detection with conditional compilation
- Modal dialog system with proper state management
- Comprehensive error handling and user feedback
- Maintains all existing functionality while adding productivity features

**Security Considerations Addressed**:
- Confirmation required for all process termination actions
- Visual warnings for potentially destructive operations
- Input validation through existing query parsing infrastructure
- Clear audit trail through status message logging

**Approved by**: Jason (Product Owner)
**Date**: September 23, 2025