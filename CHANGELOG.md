# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/linko-zhao/OxideSSH/releases/tag/v0.1.0) - 2026-11-23

### Added

- *(desktop)* simplify connection editor
- *(terminal)* align built-in colors with Codex themes
- *(desktop)* unify connection addition entry
- *(settings)* configure global terminal base colors
- *(settings)* use file-based ANSI palette picker
- *(settings)* support TOML configuration overrides
- *(discovery)* import SSH config and WSL connections
- *(ui)* support customizable file-based themes
- *(settings)* import export and back up configuration
- *(settings)* configure local shell
- *(settings)* add copy on select
- *(terminal)* add configurable OSC 52 clipboard policy
- *(settings)* add terminal bell modes
- *(settings)* configure SSH keepalive
- *(settings)* add SSH connection timeout
- *(settings)* clear all application credentials
- *(profile)* add startup command
- *(settings)* import and export trusted hosts
- *(terminal)* add configurable OSC 52 clipboard policy
- *(profile)* configure remote environment variables
- *(settings)* configure SSH terminal type
- *(settings)* configure terminal font metrics
- *(settings)* add SSH compression preference
- *(settings)* remember window size
- *(settings)* add SSH connection timeout
- *(settings)* configure SSH keepalive
- *(settings)* add copy on select
- *(settings)* configure terminal cursor style and blink
- *(settings)* add active session close confirmation setting
- *(settings)* add terminal bell modes
- *(settings)* 优化设置页面布局与选择控件
- *(settings)* 新增新建标签按钮默认行为选项
- *(tags)* 优化管理标签入口与对话框交互
- *(settings)* render trusted hosts as cards with red trash delete button
- *(settings)* 设置界面改为左侧分类栏+右侧内容的两栏布局
- *(desktop)* set macOS Dock icon at runtime for bare binaries
- *(ui)* add pill spacing, borders, and workspace separator to tab strip
- *(ui)* merge tab strip into custom-drawn title bar on Windows/macOS
- *(session)* add local shell terminal via portable-pty
- *(ui)* switch palette to Claude-style warm tones
- *(ui)* unify rounded-card layout for sidebar and terminal
- make untagged filter mutually exclusive with tag selection
- add host tags with workspace filter sidebar
- redesign UI with soft slate palette and rounded components
- restyle tab strip with segmented variant and fixed-width tabs
- add Termius-style workspace page with permanent tab strip
- migrate desktop client to gpui-component
- initial OxideSSH SSH client for Windows and macOS

### Fixed

- 统一按钮字号并修复主按钮内容不可见
- *(terminal)* forward control keys to sessions
- *(terminal)* clear click-only selection
- *(terminal)* align graphemes to grid columns
- *(ci)* cfg-gate platform discovery
- *(build)* streamline local preview setup
- *(terminal)* forward mouse events to applications
- route terminal clipboard actions by focus
- prevent settings overflow in narrow windows
- restore settings page scrolling
- *(ci)* address lint and audit failures
- *(ci)* format Rust sources
- *(terminal)* preserve remote cursor command precedence
- *(terminal)* complete cursor blink and remote control
- *(terminal)* measure each connecting tab before startup
- *(terminal)* start sessions with measured geometry
- *(settings)* complete roadmap integration
- *(terminal)* expire visual bell prompt
- *(settings)* preserve merged integration behavior
- *(terminal)* blink cursor and reset remote state
- *(settings)* preserve new tab action setter
- *(terminal)* expire visual bell prompt
- *(ci)* 修复格式检查与依赖审计
- *(ci)* format settings view
- *(terminal)* 修复 macOS 下直接回车无效、必须 Ctrl+Enter 的问题
- *(macos)* make Cmd+Q quit the app and add a New Window dock menu item
- *(dialog)* cap dialog height at 80% of viewport and scroll overflowing content
- *(macos)* show correct app icon in Finder and match Dock icon size
- *(macos)* pad title bar strip so tab margin matches and traffic lights aren't clipped
- *(ui)* repair Windows title bar drag, maximize/restore, and tab overflow
- consume must_use result of embed_resource::compile
- embed application icon into Windows executable
- repair CI formatting and audit permissions
- show sidebar only in workspace tab and use pill tab style
- hide Windows console window
- configure desktop packaging in CI
- dismiss error banner manually or after timeout

## [0.1.0](https://github.com/linko-zhao/OxideSSH/releases/tag/v0.1.0) - 2026-11-23

### Added

- *(terminal)* cut over exclusively to Ghostty
- *(terminal)* add configurable OSC 52 clipboard policy
- *(settings)* configure SSH keepalive
- *(profile)* add startup command
- *(session)* add local shell terminal via portable-pty
- add host tags with workspace filter sidebar

### Fixed

- *(terminal)* forward mouse events to applications
- *(ci)* address lint and audit failures

## [0.1.0](https://github.com/linko-zhao/OxideSSH/releases/tag/v0.1.0) - 2026-11-23

### Added

- *(terminal)* cut over exclusively to Ghostty
- *(terminal)* add configurable OSC 52 clipboard policy
- initial OxideSSH SSH client for Windows and macOS

### Fixed

- *(terminal)* forward control keys to sessions
- *(terminal)* align graphemes to grid columns
- *(build)* streamline local preview setup
- *(terminal)* forward mouse events to applications
- *(ci)* expose Ghostty dylib to macOS tests
- *(terminal)* link Ghostty static library on macOS
- fix terminal Ghostty review findings
- *(terminal)* preserve remote cursor command precedence
- *(terminal)* complete cursor blink and remote control
- *(settings)* complete roadmap integration

## [0.1.0](https://github.com/linko-zhao/OxideSSH/releases/tag/v0.1.0) - 2026-11-23

### Added

- *(settings)* configure global terminal base colors
- *(settings)* use file-based ANSI palette picker
- *(settings)* support TOML configuration overrides
- *(discovery)* import SSH config and WSL connections
- *(ui)* support customizable file-based themes
- *(settings)* import export and back up configuration
- *(settings)* configure local shell
- *(settings)* add copy on select
- *(terminal)* add configurable OSC 52 clipboard policy
- *(settings)* add terminal bell modes
- *(settings)* configure SSH keepalive
- *(settings)* add SSH connection timeout
- *(settings)* clear all application credentials
- *(profile)* add startup command
- *(settings)* import and export trusted hosts
- *(terminal)* add configurable OSC 52 clipboard policy
- *(profile)* configure remote environment variables
- *(settings)* configure SSH terminal type
- *(settings)* configure terminal font metrics
- *(settings)* add SSH compression preference
- *(settings)* remember window size
- *(settings)* add SSH connection timeout
- *(settings)* configure SSH keepalive
- *(settings)* add copy on select
- *(settings)* add active session close confirmation setting
- *(settings)* add terminal bell modes
- *(settings)* 新增新建标签按钮默认行为选项
- *(local)* prefer PowerShell over cmd.exe for Windows local terminal
- *(session)* add local shell terminal via portable-pty
- add host tags with workspace filter sidebar
- initial OxideSSH SSH client for Windows and macOS

### Fixed

- *(ci)* update vulnerable SSH dependencies
- *(ci)* resolve clippy warnings
- *(build)* streamline local preview setup
- *(ci)* address lint and audit failures
- *(terminal)* complete cursor blink and remote control
- *(terminal)* start sessions with measured geometry
- *(settings)* complete roadmap integration
- *(settings)* restore integrated defaults
- *(settings)* preserve merged integration behavior
- compile SSH test fixture on Windows
- gate Unix-only SSH fixture state
- configure desktop packaging in CI
