#!/usr/bin/env python3
"""
Rust 代码统计脚本

统计项目中的：
1. 有效代码行数（不含注释、空行）
2. 单元测试数量
3. 集成测试数量
"""

import os
import re
from pathlib import Path
from typing import Dict, List, Tuple


class RustCodeStats:
    """Rust 代码统计器"""

    # 注释正则表达式
    LINE_COMMENT = re.compile(r'^\s*//')
    BLOCK_COMMENT_START = re.compile(r'/\*')
    BLOCK_COMMENT_END = re.compile(r'\*/')

    # 测试函数正则表达式
    TEST_FN = re.compile(r'^\s*#\[test\]\s*$')
    TEST_ATTR_FN = re.compile(r'^\s*#\[.*test.*\]\s*$')
    FN_DECL = re.compile(r'^\s*(pub\s+)?(async\s+)?(unsafe\s+)?fn\s+(\w+)\s*\(')

    def __init__(self, project_root: str):
        self.project_root = Path(project_root)

    def is_in_block_comment(self, line: str, in_block: bool) -> bool:
        """检查是否在块注释中"""
        if in_block:
            if self.BLOCK_COMMENT_END.search(line):
                # 检查注释结束后是否还有代码
                after_comment = self.BLOCK_COMMENT_END.split(line, 1)[1] if self.BLOCK_COMMENT_END.split(line, 1)[1:] else ""
                return not after_comment.strip() or self.BLOCK_COMMENT_START.search(after_comment)
            return True
        return False

    def count_effective_lines(self, content: str) -> int:
        """统计有效代码行数（排除注释和空行）"""
        lines = content.split('\n')
        count = 0
        in_block_comment = False

        for line in lines:
            # 检查块注释
            if self.BLOCK_COMMENT_START.search(line):
                in_block_comment = True
                # 检查是否在同一行结束
                if self.BLOCK_COMMENT_END.search(line):
                    in_block_comment = False
                    # 检查注释后是否有代码
                    after = self.BLOCK_COMMENT_END.split(line, 1)[1] if self.BLOCK_COMMENT_END.split(line, 1)[1:] else ""
                    if after.strip() and not self.LINE_COMMENT.search(after):
                        count += 1
                continue

            if in_block_comment:
                if self.BLOCK_COMMENT_END.search(line):
                    in_block_comment = False
                    # 检查注释后是否有代码
                    after = self.BLOCK_COMMENT_END.split(line, 1)[1] if self.BLOCK_COMMENT_END.split(line, 1)[1:] else ""
                    if after.strip() and not self.LINE_COMMENT.search(after):
                        count += 1
                continue

            # 跳过行注释
            if self.LINE_COMMENT.search(line):
                continue

            # 跳过空行
            if not line.strip():
                continue

            count += 1

        return count

    def count_tests_in_file(self, content: str) -> int:
        """统计文件中的测试数量"""
        lines = content.split('\n')
        count = 0
        next_is_test_fn = False

        for line in lines:
            if next_is_test_fn:
                if self.FN_DECL.search(line):
                    count += 1
                next_is_test_fn = False
                continue

            if self.TEST_FN.search(line):
                next_is_test_fn = True
            # 处理像 #[serial] 这样的测试属性
            elif re.search(r'^\s*#\[.*\]', line):
                # 检查下一行是否是 #[test]
                pass

        return count

    def count_tests_with_serial(self, content: str) -> int:
        """统计测试数量（支持 #[serial] 等属性）"""
        lines = content.split('\n')
        count = 0
        test_attrs = 0

        for line in lines:
            # 检查测试属性
            if re.search(r'^\s*#\[test\]', line):
                test_attrs += 1
            elif re.search(r'^\s*#\[serial\]', line) or re.search(r'^\s*#\[tokio::test\]', line):
                test_attrs += 1
            elif self.FN_DECL.search(line):
                # 如果前面有测试属性，这可能是测试函数
                if test_attrs > 0:
                    count += 1
                    test_attrs = 0
            elif not re.search(r'^\s*#\[', line):
                # 遇到非属性行，重置
                test_attrs = 0

        return count

    def find_rust_files(self, directory: Path, pattern: str = "*.rs") -> List[Path]:
        """查找目录下的所有 Rust 文件"""
        return list(directory.rglob(pattern))

    def run(self) -> Dict:
        """运行统计"""
        results = {
            "code_lines": 0,
            "unit_tests": 0,
            "integration_tests": 0,
            "details": {
                "src": {},
                "tests": {}
            }
        }

        # 统计 src 目录（源代码 + 单元测试）
        src_dir = self.project_root / "src"
        if src_dir.exists():
            for rust_file in self.find_rust_files(src_dir):
                content = rust_file.read_text(encoding='utf-8', errors='ignore')
                lines = self.count_effective_lines(content)
                tests = self.count_tests_with_serial(content)

                relative_path = rust_file.relative_to(self.project_root)
                results["code_lines"] += lines
                results["unit_tests"] += tests
                results["details"]["src"][str(relative_path)] = {
                    "lines": lines,
                    "tests": tests
                }

        # 统计 tests 目录（集成测试）
        tests_dir = self.project_root / "tests"
        if tests_dir.exists():
            for rust_file in self.find_rust_files(tests_dir):
                content = rust_file.read_text(encoding='utf-8', errors='ignore')
                lines = self.count_effective_lines(content)
                tests = self.count_tests_with_serial(content)

                relative_path = rust_file.relative_to(self.project_root)
                results["code_lines"] += lines
                results["integration_tests"] += tests
                results["details"]["tests"][str(relative_path)] = {
                    "lines": lines,
                    "tests": tests
                }

        # 统计 examples 目录（如果有）
        examples_dir = self.project_root / "examples"
        if examples_dir.exists():
            for rust_file in self.find_rust_files(examples_dir):
                content = rust_file.read_text(encoding='utf-8', errors='ignore')
                lines = self.count_effective_lines(content)
                results["code_lines"] += lines

        return results


def print_results(results: Dict):
    """打印统计结果"""
    print("=" * 60)
    print("Rust 代码统计报告")
    print("=" * 60)
    print()

    # 总览
    total_tests = results["unit_tests"] + results["integration_tests"]
    print(f"📊 总体统计:")
    print(f"   有效代码行数: {results['code_lines']:,}")
    print(f"   测试用例总数: {total_tests}")
    print(f"     - 单元测试: {results['unit_tests']}")
    print(f"     - 集成测试: {results['integration_tests']}")
    print()

    # src 目录详情
    if results["details"]["src"]:
        print(f"📁 src/ 目录 ({len(results['details']['src'])} 文件):")
        for file, stats in sorted(results["details"]["src"].items()):
            test_info = f", {stats['tests']} 测试" if stats['tests'] > 0 else ""
            print(f"   {stats['lines']:5d} 行  {file}{test_info}")
        print()

    # tests 目录详情
    if results["details"]["tests"]:
        print(f"📁 tests/ 目录 ({len(results['details']['tests'])} 文件):")
        for file, stats in sorted(results["details"]["tests"].items()):
            print(f"   {stats['lines']:5d} 行, {stats['tests']} 测试  {file}")
        print()

    print("=" * 60)


def main():
    """主函数"""
    # 获取项目根目录
    script_path = Path(__file__).resolve()
    project_root = script_path.parent.parent

    print(f"🔍 扫描项目: {project_root}")
    print()

    stats = RustCodeStats(str(project_root))
    results = stats.run()
    print_results(results)


if __name__ == "__main__":
    main()
