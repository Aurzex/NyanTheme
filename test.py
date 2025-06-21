# Python主题配置测试脚本
import subprocess
import time
import sys
import select
from pathlib import Path

# 配置路径
THEME_PATH = Path("theme.json")
CLI_THEME_PATH = Path("clitheme.py")

def run_test():
    """运行主题测试"""
    # 验证文件存在
    if not THEME_PATH.exists():
        print(f"错误：主题文件不存在 {THEME_PATH}")
        return
    if not CLI_THEME_PATH.exists():
        print(f"错误：clitheme.py 文件不存在 {CLI_THEME_PATH}")
        return
    
    # 构建命令
    cmd = [
        sys.executable,  # 使用当前Python解释器
        str(CLI_THEME_PATH),
        "-apply", str(THEME_PATH),
        "--", "python3", "-i", "-q"  # 添加 -q 参数减少启动信息
    ]
    
    print("启动测试环境...")
    print(f"命令: {' '.join(cmd)}")
    print("=" * 60)
    
    # 启动子进程
    with subprocess.Popen(
        cmd,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,  # 将stderr合并到stdout
        text=True,
        bufsize=1,
        universal_newlines=True
    ) as proc:
        # 等待Python启动
        time.sleep(1)
        
        # 定义测试用例
        test_cases = [
            # 基础错误
            ("print(1/0)", "ZeroDivisionError"),
            ("print(undefined_var)", "NameError"),
            ("import non_existent_module", "ModuleNotFoundError"),
            ('open("non_existent_file.txt")', "FileNotFoundError"),
            ("[1,2,3][10]", "IndexError"),
            ("{}['key']", "KeyError"),
            ("int('abc')", "ValueError"),
            ("'1' + 1", "TypeError"),
            
            # 语法错误
            ("print('hello)", "SyntaxError"),  # 缺少引号
            ("if True\n    print('yes')", "IndentationError"),  # 缺少冒号和缩进错误
            
            # 复杂错误
            ("def recursive():\n    recursive()\nrecursive()", "RecursionError"),
            ("import os\nos.remove('/etc/passwd')", "PermissionError"),
            
            # 自定义错误消息
            ("exit()", "exit"),  # 测试退出
        ]
        
        # 运行测试用例
        for code, test_name in test_cases:
            print(f"\n>>> 测试: {test_name} <<<")
            print(f"输入: {code}")
            
            # 发送代码
            if proc.stdin is not None:
                proc.stdin.write(code + "\n")
                proc.stdin.flush()
            
            # 读取输出 - 使用select避免阻塞
            output_lines = []
            start_time = time.time()
            
            while True:
                # 使用select检查是否有可读数据
                rlist, _, _ = select.select([proc.stdout], [], [], 1.0)
                
                if rlist:
                    if proc.stdout is not None:
                        line = proc.stdout.readline()
                    else:
                        break
                    if not line:
                        break
                    
                    output_lines.append(line)
                    
                    # 检查是否出现Python提示符
                    if ">>> " in line:
                        break
                else:
                    # 超时
                    if time.time() - start_time > 5:  # 5秒超时
                        print("超时: 未检测到提示符")
                        break
                    continue
            
            # 显示输出
            print("输出:")
            for line in output_lines:
                print(line.rstrip())
            
            # 添加分隔线
            print("-" * 60)
        
        # 结束进程
        if proc.stdin is not None:
            proc.stdin.write("exit()\n")
            proc.stdin.flush()
        proc.wait(timeout=3)

if __name__ == "__main__":
    print("=" * 60)
    print("Python主题配置测试")
    print("=" * 60)
    run_test()
    print("测试完成!")