use super::action::{resolve_url, write_files, AmalgamationAction};

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Terminal,
};
use std::{
    fs,
    io::stdout,
    path::PathBuf,
};

pub async fn run_interactive_mode(action: AmalgamationAction) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Download and extract files first
    let temp_dir = tempfile::TempDir::new()?;
    let resolved_url = resolve_url(&action.url);
    let zip_content = action.download_repository(&resolved_url).await?;
    action.extract_zip(&zip_content, &temp_dir)?;

    // Find the actual repository root directory and create a virtual root from its contents
    let repo_root = fs::read_dir(temp_dir.path())?
        .filter_map(Result::ok)
        .find(|entry| entry.path().is_dir())
        .ok_or_else(|| anyhow::anyhow!("Could not find repository root directory"))?
        .path();

    // Create file tree starting from the repository root
    let root = FileTreeNode::new(repo_root)?;
    let mut state = ListState::default();
    state.select(Some(0));

    let result = run_app(&mut terminal, root, &mut state, &action);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    result
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    mut root: FileTreeNode,
    state: &mut ListState,
    action: &AmalgamationAction,
) -> Result<()> {
    let mut focus_on_tree = true;

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(1),
                    Constraint::Length(3),
                    Constraint::Length(1),
                ])
                .split(f.size());

            // File tree
            let items: Vec<ListItem> = create_tree_items(&root);
            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title("Files"))
                .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
            f.render_stateful_widget(list, chunks[0], state);

            // Instructions
            let help_text =
                "TAB: Switch focus | SPACE: Select/Deselect | ENTER: Expand/Collapse | ESC: Exit";
            let help = Paragraph::new(help_text).block(Block::default().borders(Borders::ALL));
            f.render_widget(help, chunks[1]);

            // OK button
            let button_style = if !focus_on_tree {
                Style::default().fg(Color::Black).bg(Color::Green)
            } else {
                Style::default().fg(Color::Green)
            };
            let button = Paragraph::new("[ OK ]")
                .style(button_style)
                .block(Block::default());
            f.render_widget(button, chunks[2]);
        })?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => {
                    return Ok(());
                }
                KeyCode::Enter => {
                    if focus_on_tree {
                        if let Some(selected) = state.selected() {
                            toggle_node_expansion(&mut root, selected);
                        }
                    } else {
                        // OK button pressed - process selected files
                        if let Err(e) = root.write_selected_files(&action.output_pathname) {
                            eprintln!("Error writing files: {}", e);
                        }
                        return Ok(());
                    }
                }
                KeyCode::Tab => {
                    focus_on_tree = !focus_on_tree;
                }
                KeyCode::Char(' ') => {
                    if focus_on_tree {
                        if let Some(selected) = state.selected() {
                            toggle_node_selection(&mut root, selected);
                        }
                    }
                }
                KeyCode::Up => {
                    if focus_on_tree {
                        let current = state.selected().unwrap_or(0);
                        state.select(Some(current.saturating_sub(1)));
                    }
                }
                KeyCode::Down => {
                    if focus_on_tree {
                        let current = state.selected().unwrap_or(0);
                        let max_index = count_visible_nodes(&root) - 1;
                        state.select(Some(std::cmp::min(current + 1, max_index)));
                    }
                }
                _ => {}
            }
        }
    }
}

pub struct FileTreeNode {
    path: PathBuf,
    is_dir: bool,
    is_selected: bool,
    is_expanded: bool,
    children: Vec<FileTreeNode>,
}

impl FileTreeNode {
    fn new(path: PathBuf) -> Result<Self> {
        let is_dir = path.is_dir();
        let children = if is_dir {
            let mut children = Vec::new();
            for entry in fs::read_dir(&path)? {
                let entry = entry?;
                children.push(FileTreeNode::new(entry.path())?);
            }
            children.sort_by_key(|node| (node.is_dir, node.path.to_string_lossy().into_owned()));
            children
        } else {
            Vec::new()
        };

        Ok(FileTreeNode {
            path,
            is_dir,
            is_selected: true,
            is_expanded: false,
            children,
        })
    }

    fn collect_selected_files(&self) -> Vec<PathBuf> {
        let mut selected = Vec::new();
        if !self.is_dir && self.is_selected {
            selected.push(self.path.clone());
        }
        if self.is_dir {
            for child in &self.children {
                selected.extend(child.collect_selected_files());
            }
        }
        selected
    }

    fn write_selected_files(&self, output_path: &PathBuf) -> Result<()> {
        let selected_files = self.collect_selected_files();
        if selected_files.is_empty() {
            return Ok(());
        }

        write_files(&selected_files, output_path)
    }
}

fn create_tree_items(node: &FileTreeNode) -> Vec<ListItem> {
    fn build_items(node: &FileTreeNode, depth: usize, items: &mut Vec<ListItem>) {
        let prefix = "  ".repeat(depth);
        let icon = if node.is_dir {
            if node.is_expanded {
                "▼ "
            } else {
                "▶ "
            }
        } else {
            "  "
        };

        let checkbox = if node.is_selected { "[✓]" } else { "[ ]" };
        let name = node.path.file_name().unwrap_or_default().to_string_lossy();

        items.push(ListItem::new(format!(
            "{}{}{} {}",
            prefix, icon, checkbox, name
        )));

        if node.is_expanded {
            for child in &node.children {
                build_items(child, depth + 1, items);
            }
        }
    }

    let mut items = Vec::new();
    build_items(node, 0, &mut items);
    items
}

fn count_visible_nodes(node: &FileTreeNode) -> usize {
    let mut count = 1;
    if node.is_expanded {
        count += node.children.iter().map(count_visible_nodes).sum::<usize>();
    }
    count
}

fn toggle_node_expansion(root: &mut FileTreeNode, index: usize) {
    fn toggle_recursive(node: &mut FileTreeNode, index: &mut usize) -> bool {
        if *index == 0 {
            if node.is_dir {
                node.is_expanded = !node.is_expanded;
            }
            return true;
        }
        *index -= 1;

        if node.is_expanded {
            for child in &mut node.children {
                if toggle_recursive(child, index) {
                    return true;
                }
            }
        }
        false
    }

    let mut current_index = index;
    toggle_recursive(root, &mut current_index);
}

fn toggle_node_selection(root: &mut FileTreeNode, index: usize) {
    fn toggle_recursive(node: &mut FileTreeNode, index: &mut usize) -> bool {
        if *index == 0 {
            node.is_selected = !node.is_selected;
            // If it's a directory, propagate selection to children
            if node.is_dir {
                for child in &mut node.children {
                    child.is_selected = node.is_selected;
                }
            }
            return true;
        }
        *index -= 1;

        if node.is_expanded {
            for child in &mut node.children {
                if toggle_recursive(child, index) {
                    return true;
                }
            }
        }
        false
    }

    let mut current_index = index;
    toggle_recursive(root, &mut current_index);
}
