#!/usr/bin/env python3
"""
Blockchain Engineer CLI Tool
Single command interface for Kimura blockchain development

USAGE:
    python3 blockchain.py build --mode release
    python3 blockchain.py test --suite all
    python3 blockchain.py git commit --message "Add feature"
    python3 blockchain.py git pr --title "Feature PR"
"""

import argparse
import sys
import subprocess
import os
from pathlib import Path

# Project path - the chain directory itself
PROJECT_ROOT = Path(__file__).parent

def validate_working_directory():
    """Ensure we're running from the chain directory"""
    current_dir = Path.cwd()
    if not (current_dir / 'blockchain.py').exists():
        print("ERROR: Must run blockchain.py from the chain/ directory")
        print(f"Current directory: {current_dir}")
        print(f"Expected: {PROJECT_ROOT}")
        sys.exit(1)

def run_in_project(cmd, cwd=None):
    """Run command in the blockchain project directory"""
    work_dir = cwd or PROJECT_ROOT
    try:
        result = subprocess.run(cmd, cwd=work_dir, check=True, capture_output=True, text=True)
        if result.stdout:
            print(result.stdout)
        return True
    except subprocess.CalledProcessError as e:
        print(f"Error: {e}")
        if e.stderr:
            print(e.stderr)
        return False

def main():
    # Validate we're in the right directory
    validate_working_directory()
    
    parser = argparse.ArgumentParser(
        prog='blockchain',
        description='CLI for Kimura blockchain development',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog='''
Engineer Commands:
  python3 blockchain.py build --mode release    Build the project
  python3 blockchain.py test --suite all        Run all tests
  python3 blockchain.py git commit -m "msg"     Commit changes
  python3 blockchain.py git pr --title "PR"     Create pull request
  python3 blockchain.py git branch --name X     Create branch
  python3 blockchain.py git issue --title X     Create GitHub issue

For detailed help: python3 blockchain.py <command> --help
        '''
    )
    
    subparsers = parser.add_subparsers(dest='command', help='Available commands')
    
    # Build subcommand
    build_parser = subparsers.add_parser('build', help='Build blockchain components')
    build_parser.add_argument('--mode', choices=['debug', 'release'], default='debug')
    build_parser.add_argument('--target', choices=['all', 'node', 'consensus', 'storage', 'network'])
    build_parser.add_argument('--clean', action='store_true')
    build_parser.add_argument('--features', help='Comma-separated features to enable')
    
    # Test subcommand
    test_parser = subparsers.add_parser('test', help='Run tests and benchmarks')
    test_parser.add_argument('--suite', choices=['unit', 'integration', 'all'], default='all')
    test_parser.add_argument('--coverage', action='store_true')
    test_parser.add_argument('--benchmark', action='store_true')
    
    # Git/GitHub workflow subcommand
    git_parser = subparsers.add_parser('git', help='Git and GitHub workflow commands')
    git_subparsers = git_parser.add_subparsers(dest='git_action')
    
    # Issue subcommand
    git_issue = git_subparsers.add_parser('issue', help='Create GitHub issue')
    git_issue.add_argument('--title', required=True, help='Issue title')
    git_issue.add_argument('--body', help='Issue body/description')
    git_issue.add_argument('--labels', help='Comma-separated labels')
    git_issue.add_argument('--assignee', help='Assignee username')
    
    # Branch subcommand
    git_branch = git_subparsers.add_parser('branch', help='Create git branch')
    git_branch.add_argument('--name', required=True, help='Branch name')
    git_branch.add_argument('--from', dest='from_branch', default='main', help='Source branch')
    git_branch.add_argument('--checkout', action='store_true', help='Checkout after creation')
    
    # Commit subcommand
    git_commit = git_subparsers.add_parser('commit', help='Create git commit')
    git_commit.add_argument('--message', '-m', required=True, help='Commit message')
    git_commit.add_argument('--all', '-a', action='store_true', help='Stage all changes')
    git_commit.add_argument('--no-verify', action='store_true', help='Skip pre-commit hooks')
    
    # PR subcommand
    git_pr = git_subparsers.add_parser('pr', help='Create pull request')
    git_pr.add_argument('--title', required=True, help='PR title')
    git_pr.add_argument('--body', help='PR body/description')
    git_pr.add_argument('--base', default='main', help='Base branch')
    git_pr.add_argument('--draft', action='store_true', help='Create as draft')
    git_pr.add_argument('--reviewer', help='Request reviewer (username)')
    
    # Parse arguments
    args = parser.parse_args()
    
    if not args.command:
        parser.print_help()
        sys.exit(0)
    
    # Route to appropriate handler
    handle_command(args)

def handle_command(args):
    """Route commands to their handlers"""
    command = args.command
    
    if command == 'build':
        handle_build(args)
    elif command == 'test':
        handle_test(args)
    elif command == 'git':
        handle_git(args)
    else:
        print(f"Unknown command: {command}")
        sys.exit(1)

def handle_build(args):
    """Handle build commands"""
    cmd = ['cargo', 'build']
    
    if args.mode == 'release':
        cmd.append('--release')
    
    if args.target and args.target != 'all':
        cmd.extend(['-p', f'kimura-{args.target}'])
    
    if args.features:
        cmd.extend(['--features', args.features])
    
    if args.clean:
        print("Running cargo clean first...")
        if not run_in_project(['cargo', 'clean']):
            sys.exit(1)
    
    print(f"Building: {' '.join(cmd)}")
    print(f"Mode: {args.mode}")
    print(f"Target: {args.target or 'all'}")
    
    if not run_in_project(cmd):
        print("Build failed!")
        sys.exit(1)
    else:
        print("Build successful!")

def handle_test(args):
    """Handle test commands"""
    if args.suite == 'unit':
        cmd = ['cargo', 'test', '--lib']
    elif args.suite == 'integration':
        cmd = ['cargo', 'test', '-p', 'kimura-node', '--test', 'integration_tests']
    else:  # all
        cmd = ['cargo', 'test', '--workspace']
    
    if args.coverage:
        print("Coverage: requires cargo-tarpaulin (not installed by default)")
        print("Install with: cargo install cargo-tarpaulin")
        cmd = ['cargo', 'tarpaulin', '--all']
    
    if args.benchmark:
        print("Running benchmarks...")
        cmd = ['cargo', 'bench']
    
    print(f"Running test suite: {args.suite}")
    
    if not run_in_project(cmd):
        print("Tests failed!")
        sys.exit(1)
    else:
        print("Tests passed!")

def handle_git(args):
    """Handle git and GitHub workflow commands"""
    action = getattr(args, 'git_action', None)
    
    if action == 'issue':
        cmd = ['gh', 'issue', 'create', '--title', args.title]
        if args.body:
            cmd.extend(['--body', args.body])
        if args.labels:
            cmd.extend(['--label', args.labels])
        if args.assignee:
            cmd.extend(['--assignee', args.assignee])
        
        print(f"Creating GitHub issue: {args.title}")
        if not run_in_project(cmd):
            sys.exit(1)
        print("Issue created successfully!")
    
    elif action == 'branch':
        if args.checkout:
            cmd = ['git', 'checkout', '-b', args.name]
        else:
            cmd = ['git', 'branch', args.name, args.from_branch]
        
        print(f"Creating branch: {args.name} from {args.from_branch}")
        if not run_in_project(cmd):
            sys.exit(1)
        print(f"Branch '{args.name}' created successfully!")
    
    elif action == 'commit':
        if args.all:
            run_in_project(['git', 'add', '-A'])
        
        cmd = ['git', 'commit', '-m', args.message]
        if args.no_verify:
            cmd.append('--no-verify')
        
        print(f"Creating commit: {args.message}")
        if not run_in_project(cmd):
            sys.exit(1)
        print("Commit created successfully!")
    
    elif action == 'pr':
        cmd = ['gh', 'pr', 'create', '--title', args.title]
        if args.body:
            cmd.extend(['--body', args.body])
        if args.base:
            cmd.extend(['--base', args.base])
        if args.draft:
            cmd.append('--draft')
        if args.reviewer:
            cmd.extend(['--reviewer', args.reviewer])
        
        print(f"Creating pull request: {args.title}")
        if not run_in_project(cmd):
            sys.exit(1)
        print("Pull request created successfully!")
    
    else:
        print("Git action required (issue, branch, commit, or pr)")
        sys.exit(1)

if __name__ == '__main__':
    main()