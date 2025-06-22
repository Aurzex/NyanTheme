use anyhow::{anyhow, Context, Result};
use clap::Parser;
use regex::Regex;
use serde::Deserialize;
use std::{
    ffi::OsString,
    path::PathBuf,
    process::{Stdio},
    sync::Arc,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader as AsyncBufReader},
    process::Command as AsyncCommand,
    task,
};

/// 命令行输出文本替换工具
#[derive(Parser, Debug)]
#[command(
    name = "clitheme",
    version,
    author,
    about,
    long_about = "示例:\n  clitheme -apply config.json -- python3 script.py\n  clitheme -apply config.json python3 script.py",
    after_help = "示例: clitheme -apply theme.json -- python3 -i"
)]
struct Args {
    /// 包含替换规则的JSON配置文件
    #[arg(short, long, required = true)]
    apply: PathBuf,

    /// 指定使用的语言环境
    #[arg(short, long, default_value = "default")]
    locale: String,

    /// 要执行的命令及其参数
    command: Vec<OsString>,
}

/// 替换规则配置
#[derive(Debug, Deserialize)]
struct ReplacementConfig {
    pattern: String,
    replacement: String,
    #[serde(default = "default_locale")]
    locale: String,
    #[serde(default, rename = "filter_commands")]
    commands: Vec<String>,
}

fn default_locale() -> String {
    "default".to_string()
}

/// 编译后的替换规则
#[derive(Clone)]
struct ReplacementRule {
    pattern: Regex,
    replacement: String,
    locale: String,
    commands: Vec<String>,
}

impl ReplacementRule {
    fn from_config(config: &ReplacementConfig) -> Result<Self> {
        let pattern = Regex::new(&config.pattern)
            .with_context(|| format!("无效的正则表达式: {}", config.pattern))?;
        Ok(Self {
            pattern,
            replacement: config.replacement.clone(),
            locale: config.locale.clone(),
            commands: config.commands.iter().map(|s| s.to_lowercase()).collect(),
        })
    }
}

/// 加载并验证JSON配置文件
fn load_config(path: &PathBuf) -> Result<Vec<ReplacementRule>> {
    let data = std::fs::read_to_string(path)
        .with_context(|| format!("无法读取配置文件: {}", path.display()))?;

    let configs: Vec<ReplacementConfig> = serde_json::from_str(&data)
        .with_context(|| "配置文件格式错误: 根元素必须是数组".to_string())?;

    let mut rules = Vec::new();
    for config in configs {
        match ReplacementRule::from_config(&config) {
            Ok(rule) => rules.push(rule),
            Err(e) => eprintln!("警告: 跳过无效规则 - {}", e),
        }
    }
    Ok(rules)
}

/// 应用所有匹配的替换规则到文本
fn apply_replacements(
    text: &str,
    command_name: &str,
    rules: &[ReplacementRule],
    locale: &str,
) -> String {
    let command_name = command_name.to_lowercase();
    let mut result = text.to_string();

    for rule in rules {
        if !rule.commands.is_empty() && !rule.commands.contains(&command_name) {
            continue;
        }
        if rule.locale != locale {
            continue;
        }
        result = rule.pattern.replace_all(&result, &rule.replacement).to_string();
    }
    result
}

/// 处理流数据并应用替换规则
async fn process_stream<R, W>(
    reader: R,
    mut writer: W,
    command_name: &str,
    rules: &[ReplacementRule],
    locale: &str,
) -> Result<()>
where
    R: tokio::io::AsyncBufReadExt + Unpin,
    W: tokio::io::AsyncWriteExt + Unpin,
{
    let mut lines = reader.lines();
    while let Some(line) = lines.next_line().await? {
        let processed = apply_replacements(&line, command_name, rules, locale);
        writer.write_all(processed.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
    }
    Ok(())
}

/// 执行命令并处理输出
async fn execute_command(
    command: &[OsString],
    rules: &[ReplacementRule],
    locale: &str,
) -> Result<i32> {
    if command.is_empty() {
        return Err(anyhow!("必须指定要执行的命令"));
    }

    let command_name = PathBuf::from(&command[0])
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let interactive_commands = ["python", "python3", "ipython", "bash", "sh", "cmd", "zsh"];
    let is_interactive = interactive_commands
        .iter()
        .any(|&cmd| cmd == command_name.to_lowercase());

    let mut cmd = AsyncCommand::new(&command[0]);
    cmd.args(&command[1..])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    let mut child = if is_interactive {
        // 交互式命令需要shell
        let full_cmd = command
            .iter()
            .map(|s| s.to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join(" ");
        AsyncCommand::new("sh")
            .arg("-c")
            .arg(full_cmd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?
    } else {
        cmd.spawn()?
    };

    let stdout = child.stdout.take().expect("无法获取子进程stdout");
    let stderr = child.stderr.take().expect("无法获取子进程stderr");
    let stdin = child.stdin.take();

    // 共享规则引用
    let rules_arc = Arc::new(rules.to_vec());
    let locale_arc = Arc::new(locale.to_string());
    let command_name_arc = Arc::new(command_name);

    // 处理标准输出
    let stdout_handle = {
        let rules = Arc::clone(&rules_arc);
        let locale = Arc::clone(&locale_arc);
        let command_name = Arc::clone(&command_name_arc);
        task::spawn(async move {
            let reader = AsyncBufReader::new(stdout);
            let writer = tokio::io::stdout();
            process_stream(
                reader,
                writer,
                &command_name,
                &rules,
                &locale,
            )
            .await
        })
    };

    // 处理标准错误
    let stderr_handle = {
        let rules = Arc::clone(&rules_arc);
        let locale = Arc::clone(&locale_arc);
        let command_name = Arc::clone(&command_name_arc);
        task::spawn(async move {
            let reader = AsyncBufReader::new(stderr);
            let writer = tokio::io::stderr();
            process_stream(
                reader,
                writer,
                &command_name,
                &rules,
                &locale,
            )
            .await
        })
    };

    // 处理交互式输入
    let stdin_handle = if is_interactive && stdin.is_some() {
        let mut child_stdin = stdin.unwrap();
        let stdin = tokio::io::stdin();
        Some(task::spawn(async move {
            let mut reader = AsyncBufReader::new(stdin).lines();
            while let Some(line) = reader.next_line().await? {
                child_stdin.write_all(line.as_bytes()).await?;
                child_stdin.write_all(b"\n").await?;
                child_stdin.flush().await?;
            }
            Ok::<(), anyhow::Error>(())
        }))
    } else {
        None
    };

    // 等待子进程结束
    let status = child.wait().await?;

    // 等待所有任务完成
    let _ = stdout_handle.await;
    let _ = stderr_handle.await;
    if let Some(handle) = stdin_handle {
        let _ = handle.await;
    }

    Ok(status.code().unwrap_or(1))
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // 处理命令参数
    let command = if let Some(pos) = args.command.iter().position(|arg| arg == "--") {
        args.command.split_at(pos + 1).1.to_vec()
    } else {
        args.command
    };

    if command.is_empty() {
        return Err(anyhow!("必须指定要执行的命令"));
    }

    // 加载配置
    let rules = match load_config(&args.apply) {
        Ok(rules) => rules,
        Err(e) => {
            eprintln!("配置错误: {}", e);
            std::process::exit(1);
        }
    };

    // 执行命令
    match execute_command(&command, &rules, &args.locale).await {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            eprintln!("执行错误: {}", e);
            std::process::exit(1);
        }
    }
}