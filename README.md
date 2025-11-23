# ctrlset

A fast, TUI-style, modal keybinding manager built in Rust with `egui`.

`ctrlset` provides a keyboard-centric interface inspired by modal editors like Vim, allowing you to efficiently create, manage, and browse sets of keybindings for all your different applications.

---

## Features

- **Modal Vim-like Interface**: Navigate and edit with maximum speed using Normal, Insert, and Command modes.
- **Fully Configurable**: All navigation and action keys can be customized via a simple `config.toml` file.
- **Application Scoping**: Keep your keybinding lists clean by scoping them to specific applications (e.g., "VS Code", "Blender", "My Project").
- **Fuzzy Search**: Instantly find any keybinding by typing a few characters.
- **Data Management**: Easily import, export, and merge keybinding sets as simple JSON files.
- **Built-in Help**: A `:help` command provides an instant overview of all available keys and commands.

## Installation

### From Source

1.  Ensure you have Rust and Cargo installed.
2.  Clone the repository:
    ```bash
    git clone https://github.com/ttyxd/ctrlset.git
    cd ctrlset
    ```
3.  Build and install the binary:
    ```bash
    cargo install --path .
    ```

## Usage

Simply run the application from your terminal:

```bash
ctrlset
```

To enable input debugging, which prints key presses to the console:

```bash
ctrlset --debug
```

## Keybindings & Commands

`ctrlset` uses a modal interface. The default keybindings are listed below and can be fully customized.

### Normal Mode

This is the default mode for navigation and issuing commands.

| Key(s)              | Action                                   |
| ------------------- | ---------------------------------------- |
| `j`/`k`             | Move selection up/down                   |
| `h`/`l`/`b`/`w`/`e` | Move selection left/right                |
| `gg`                | Go to the top of the list                |
| `G`                 | Go to the bottom of the list             |
| `i`                 | Enter **Insert Mode** to edit a cell     |
| `o`                 | Insert a new row below the cursor        |
| `O`                 | Insert a new row above the cursor        |
| `/`                 | Enter **Search Mode**                    |
| `:`                 | Enter **Command Mode**                   |
| `u`                 | Undo the last action                     |
| `dd`                | Delete the current row                   |
| `dj`                | Delete the current row and the one below |
| `dk`                | Delete the current row and the one above |
| `<Space>f`          | Open the application filter popup        |
| `<Space>e`          | Open the export menu                     |
| `<Space>i`          | Open the import menu                     |

### Command Mode

Press `:` in Normal Mode to enter Command Mode.

| Command       | Action                                    |
| ------------- | ----------------------------------------- |
| `:w`          | Save the current application's keybinds   |
| `:wq`         | Save and quit                             |
| `:q`          | Quit (fails if there are unsaved changes) |
| `:q!`         | Force quit without saving                 |
| `:new <name>` | Create a new application keybinding set   |
| `:help`       | Show the in-app help window               |

### Insert Mode

Press `i` to enter. This mode is for text entry.

| Key      | Action                                     |
| -------- | ------------------------------------------ |
| `Enter`  | Save the changes and return to Normal Mode |
| `Escape` | Save the changes and return to Normal Mode |

## Configuration

On its first run, `ctrlset` will create a configuration file at:

- **Linux/macOS**: `~/.config/ctrlset/config.toml`
- **Windows**: `C:\Users\<YourUser>\AppData\Roaming\ctrlset\ctrlset\config\config.toml`

This file is pre-populated with all the default keybindings. You can edit this file to customize every action to your liking.

## License

This project is licensed under the MIT License.

&copy; 2025 ttyxd
