# Nimble GUI Goals

## Guide for Copilot
- Do not append comments at the end of lines. That is untidy
- Do not add comments which reference a change that was made in the automatic edit, such as 'Added this line' or 'Add new parameter' This is not helpful and leaves untidy comments throughout the codebase.

## Overview
Create a GUI for the Nimble project to provide an interface for setting up parameters for commands and getting feedback as they run and process.

## Features
- **Command Setup Interface**
  - Provide input fields for setting up parameters for the `Sync`, `GenSrf`, and `Launch` commands.
  - Include options for specifying paths, URLs, and other necessary parameters.
  - Store configurations for the parameter fields so that when the application is launched, it auto-populates the fields.

- **Command Execution Feedback**
  - Display real-time feedback as commands are executed.
  - Show progress bars, status messages, and any errors encountered during execution.
  - Run tasks in the background to prevent the application from locking up.
  - Clearly switch UI states as tasks are performed to prevent conflicting tasks from being executed.

- **User-Friendly Design**
  - Ensure the interface is intuitive and easy to use.
  - Provide clear labels and instructions for each input field and command.

- **Cross-Platform Compatibility**
  - Ensure the GUI works on Windows, macOS, and Linux.

- **Modular Design**
  - Write the UI in a modular way with clear distinctions for the sections of the UI.
  - Create individual files for different sections and components of the UI.

## User Flow
1. **Server Connection**
   - User enters repository URL
   - Application connects to server and validates repository
   - Repository information is displayed (version, mod counts)

2. **Mod Management**
   - User selects local mods directory
   - Options to sync mods or launch directly
   - Clear feedback on sync progress and status

3. **Game Launch**
   - Launch with validated mod configuration
   - Use server-defined launch parameters
   - Handle platform-specific requirements

## Implementation Steps
1. **Choose a GUI Framework**
   - Use the `egui` Rust GUI framework.

2. **Set Up Project Structure**
   - Create a new Rust project for the GUI.
   - Integrate the existing Nimble project as a library.

3. **Design the Interface**
   - Create mockups or wireframes for the GUI.
   - Implement the interface using `egui`.

4. **Implement Command Setup**
   - Add input fields and controls for setting up command parameters.
   - Validate user input and handle errors gracefully.
   - Store and load configurations for parameter fields.

5. **Implement Command Execution**
   - Integrate the existing command execution logic from Nimble.
   - Display real-time feedback and progress.
   - Run tasks in the background and manage UI state transitions.
   - Avoid using async and use channels instead of Arc<Mutex> for concurrency.

6. **Testing and Debugging**
   - Test the GUI on different platforms.
   - Fix any bugs and ensure smooth operation.

7. **Documentation**
   - Provide documentation for using the GUI.
   - Include instructions for building and running the GUI on different platforms.

## Project Structure
- **Core Components**
  - `app_state.rs`: Application state management and configuration
    - Track server connection state
    - Manage repository data
    - Handle sync and launch operations
  
  - `gui.rs`: Main GUI setup and window management
    - Implement eframe::App trait
    - Handle window setup and rendering
    - Manage panel layouts and transitions
  
  - `config.rs`: Configuration storage and loading
    - Store repository URLs
    - Remember last used paths
    - Save window preferences

- **UI Components**
  - `panels/server_panel/`:
    - `mod.rs`: Panel coordination and state management
    - `connection_view.rs`: Server connection UI and logic
    - `repository_view.rs`: Repository information display
    - `action_bar.rs`: Sync and launch buttons
    - `progress_view.rs`: Operation progress display
  
  - `panels/srf_panel.rs`:
    - Display SRF generation progress
    - Show file scanning status
    - Present mod validation results

  - `widgets/`:
    - `status_display.rs`: Error and status messages
    - `path_picker.rs`: Directory selection
    - `progress_bar.rs`: Enhanced progress visualization
    - `repository_info.rs`: Repository metadata display
    - `mod_list.rs`: Required/Optional mod listing

## Communication Flow
- **Server Connection**
  - Initial connection request
  - Repository validation
  - Manifest parsing and display
  - Error handling and retry logic

- **Sync Operations**
  - Repository diff calculation
  - File download progress
  - Validation and checksums
  - Status updates

- **Launch Operations**
  - Parameter validation
  - Mod list verification
  - Launch execution
  - Status feedback

## Integration Points
- **Sync Command**
  - Hook into diff_repo() for mod status
  - Monitor execute_command_list() for downloads
  - Track ModCache updates

- **Launch Command**
  - Use generate_mod_args() for parameter display
  - Monitor launch status
  - Show Proton/Windows path handling

- **GenSRF Command**
  - Track gen_srf_for_mod() progress
  - Monitor walkdir operations
  - Show checksums and validation

## Progress Display
- **Download Operations**
  - Current file: name, size, progress
  - Overall progress: x/y files, total size
  - Speed and ETA from indicatif integration
  - Error handling and retry options

- **Status Indicators**
  - Color coding:
    - Green: Success/Complete
    - Yellow: In Progress/Warning
    - Red: Error/Failed
  - Progress bars:
    - File operations
    - Network operations
    - Overall task progress

## Component Responsibilities

- **ConnectionView**
  - URL input field
  - Connection validation
  - Connect button
  - Connection status display

- **RepositoryView**
  - Repository name and version
  - Mod counts and details
  - Server information
  - Launch parameters

- **ActionBar**
  - Path selection for mods
  - Sync button with validation
  - Launch button with validation
  - Operation status feedback

- **ProgressView**
  - Operation type indicator
  - Progress bar with percentage
  - Current file/action display
  - Cancel operation option

## Component Communication
- Parent components pass down:
  - State references
  - Message senders
  - Configuration
- Child components emit:
  - User actions
  - Validation results
  - Progress updates

## Notes
- Monitor thread channels instead of using async
- Keep UI responsive during operations
- Preserve error context for user feedback
- Show operation state transitions clearly

