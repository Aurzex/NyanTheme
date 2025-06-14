# NyanTheme - 命令行猫娘主题美化工具


### 项目描述
NyanTheme 是一个为命令行终端添加可爱猫娘风格主题的工具，特别为 Python 开发者提供错误信息的萌化转换，让枯燥的命令行体验变得更加有趣和温馨。

## 主要功能

🌈 **高度可定制**：
- 通过 JSON 配置文件自定义替换规则
- 支持多语言环境切换
- 可针对特定命令应用不同主题

⚡ **无缝集成**：
- 实时处理命令输出
- 支持交互式命令（Python, bash, zsh 等）
- 保持终端原始功能不受影响

## 使用示例

```bash
# 应用猫娘主题运行 Python
clitheme.py -apply theme.json -- python3

# 普通 Python 错误
>>> print(undefined_variable)
NameError: name 'undefined_variable' is not defined

# 猫娘主题 Python 错误
>>> print(undefined_variable)
(NameError): 主人尝试访问的"undefined_variable"不存在呢～真是个杂鱼♡～
```

## 安装使用

```bash
# 克隆仓库
git clone https://github.com/aurzex/NyanTheme.git
cd NyanTheme

# 运行示例
python clitheme.py -apply theme.json -- python3
```

## 配置文件示例

```json
{
  "replacements": [
    {
      "filter_commands": ["python", "python3"],
      "pattern": "^Traceback .*",
      "replacement": "杂鱼♡～主人的代码出错了呢～真是个杂鱼♡～："
    },
    {
      "filter_commands": ["python", "python3"],
      "pattern": "NameError: name .*",
      "replacement": "(NameError): 主人尝试访问的对象不存在呢～真是个杂鱼♡～"
    }
    // 更多可爱规则...
  ]
}
```

## 贡献指南

欢迎提交 issue 和 pull request！贡献内容包括：
- 添加新的猫娘风格提示
- 支持更多编程语言的错误美化
- 改进正则表达式匹配规则
- 翻译多语言版本
  
## 参考仓库
https://github.com/swiftycode256/clitheme
