import * as vscode from 'vscode';
import { CcgoTaskProvider } from './providers/taskProvider';
import { DependencyTreeProvider } from './views/dependencyTreeView';

let taskProvider: vscode.Disposable | undefined;
let dependencyTreeProvider: DependencyTreeProvider | undefined;

export async function activate(context: vscode.ExtensionContext) {
    console.log('CCGO extension is now active');

    // Check for CCGO.toml in workspace
    const ccgoTomlFiles = await vscode.workspace.findFiles('**/CCGO.toml', '**/node_modules/**', 1);
    const projectDetected = ccgoTomlFiles.length > 0;

    // Set context for when clauses
    vscode.commands.executeCommand('setContext', 'ccgo.projectDetected', projectDetected);

    // Register task provider
    taskProvider = vscode.tasks.registerTaskProvider('ccgo', new CcgoTaskProvider());
    context.subscriptions.push(taskProvider);

    // Register dependency tree view
    dependencyTreeProvider = new DependencyTreeProvider();
    const treeView = vscode.window.createTreeView('ccgoDependencies', {
        treeDataProvider: dependencyTreeProvider,
        showCollapseAll: true
    });
    context.subscriptions.push(treeView);

    // Register commands
    context.subscriptions.push(
        vscode.commands.registerCommand('ccgo.build', () => runCcgoCommand('build')),
        vscode.commands.registerCommand('ccgo.buildPlatform', () => buildForPlatform()),
        vscode.commands.registerCommand('ccgo.refreshDependencies', () => dependencyTreeProvider?.refresh()),
        vscode.commands.registerCommand('ccgo.install', () => runCcgoCommand('install')),
        vscode.commands.registerCommand('ccgo.test', () => runCcgoCommand('test')),
        vscode.commands.registerCommand('ccgo.bench', () => runCcgoCommand('bench')),
        vscode.commands.registerCommand('ccgo.clean', () => runCcgoCommand('clean', ['-y'])),
        vscode.commands.registerCommand('ccgo.doc', () => docCommand()),
        vscode.commands.registerCommand('ccgo.check', () => runCcgoCommand('check')),
        vscode.commands.registerCommand('ccgo.publish', () => publishCommand()),
        vscode.commands.registerCommand('ccgo.tag', () => tagCommand()),
        vscode.commands.registerCommand('ccgo.package', () => runCcgoCommand('package')),
        vscode.commands.registerCommand('ccgo.generateIdeProject', () => generateIdeProject()),
        vscode.commands.registerCommand('ccgo.tree', () => treeCommand())
    );

    // Watch CCGO.toml for changes
    const watcher = vscode.workspace.createFileSystemWatcher('**/CCGO.toml');
    watcher.onDidChange(() => {
        const autoRefresh = vscode.workspace.getConfiguration('ccgo').get('autoRefreshDependencies', true);
        if (autoRefresh) {
            dependencyTreeProvider?.refresh();
        }
    });
    watcher.onDidCreate(() => {
        vscode.commands.executeCommand('setContext', 'ccgo.projectDetected', true);
        dependencyTreeProvider?.refresh();
    });
    watcher.onDidDelete(async () => {
        const files = await vscode.workspace.findFiles('**/CCGO.toml', '**/node_modules/**', 1);
        vscode.commands.executeCommand('setContext', 'ccgo.projectDetected', files.length > 0);
        dependencyTreeProvider?.refresh();
    });
    context.subscriptions.push(watcher);

    // Initial refresh if project detected
    if (projectDetected) {
        dependencyTreeProvider.refresh();
    }
}

export function deactivate() {
    if (taskProvider) {
        taskProvider.dispose();
    }
}

async function runCcgoCommand(command: string, args: string[] = []) {
    const ccgoPath = vscode.workspace.getConfiguration('ccgo').get('executablePath', 'ccgo');
    const workspaceFolder = vscode.workspace.workspaceFolders?.[0];

    if (!workspaceFolder) {
        vscode.window.showErrorMessage('No workspace folder open');
        return;
    }

    const terminal = vscode.window.createTerminal({
        name: `CCGO: ${command}`,
        cwd: workspaceFolder.uri.fsPath
    });

    terminal.show();
    terminal.sendText(`${ccgoPath} ${command} ${args.join(' ')}`);
}

async function buildForPlatform() {
    const platforms = ['android', 'ios', 'macos', 'linux', 'windows', 'ohos', 'kmp'];
    const defaultPlatform = vscode.workspace.getConfiguration('ccgo').get('defaultPlatform');

    const platform = await vscode.window.showQuickPick(platforms, {
        placeHolder: 'Select target platform',
        title: 'CCGO Build Platform'
    });

    if (platform) {
        const options = await vscode.window.showQuickPick([
            { label: 'Debug', description: 'Build without optimizations' },
            { label: 'Release', description: 'Build with optimizations', picked: true },
            { label: 'IDE Project', description: 'Generate IDE project instead of building' }
        ], {
            placeHolder: 'Select build type',
            title: 'Build Options'
        });

        if (options) {
            const args: string[] = [platform];

            if (options.label === 'Release') {
                args.push('--release');
            } else if (options.label === 'IDE Project') {
                args.push('--ide-project');
            }

            runCcgoCommand('build', args);
        }
    }
}

async function generateIdeProject() {
    const platforms = ['ios', 'macos', 'linux', 'windows'];

    const platform = await vscode.window.showQuickPick(platforms, {
        placeHolder: 'Select platform for IDE project',
        title: 'Generate IDE Project'
    });

    if (platform) {
        runCcgoCommand('build', [platform, '--ide-project']);
    }
}

async function docCommand() {
    const options = await vscode.window.showQuickPick([
        { label: 'Generate Only', description: 'Generate documentation without opening' },
        { label: 'Generate and Open', description: 'Generate and open in browser', picked: true }
    ], {
        placeHolder: 'Select documentation option',
        title: 'Generate Documentation'
    });

    if (options) {
        if (options.label === 'Generate and Open') {
            runCcgoCommand('doc', ['--open']);
        } else {
            runCcgoCommand('doc');
        }
    }
}

async function publishCommand() {
    const targets = [
        { label: 'android', description: 'Publish to Maven repository' },
        { label: 'ios', description: 'Publish iOS framework' },
        { label: 'macos', description: 'Publish macOS framework' },
        { label: 'ohos', description: 'Publish to OHPM repository' },
        { label: 'maven', description: 'Publish to Maven' },
        { label: 'cocoapods', description: 'Publish to CocoaPods' },
        { label: 'spm', description: 'Publish to Swift Package Manager' },
        { label: 'index', description: 'Publish to package index' }
    ];

    const target = await vscode.window.showQuickPick(targets, {
        placeHolder: 'Select publish target',
        title: 'Publish Package'
    });

    if (target) {
        runCcgoCommand('publish', [target.label]);
    }
}

async function tagCommand() {
    const version = await vscode.window.showInputBox({
        prompt: 'Enter version tag (leave empty to use version from CCGO.toml)',
        placeHolder: 'e.g., 1.0.0 or v1.0.0'
    });

    if (version !== undefined) {
        if (version) {
            runCcgoCommand('tag', [version]);
        } else {
            runCcgoCommand('tag');
        }
    }
}

async function treeCommand() {
    runCcgoCommand('tree');
}
