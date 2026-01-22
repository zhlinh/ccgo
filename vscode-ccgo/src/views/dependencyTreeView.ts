import * as vscode from 'vscode';
import * as cp from 'child_process';
import * as path from 'path';

interface DependencyNode {
    name: string;
    version: string;
    source?: string;
    path?: string;
    dependencies?: DependencyNode[];
}

interface TreeOutput {
    root: DependencyNode;
    total_count: number;
    direct_count: number;
}

export class DependencyTreeProvider implements vscode.TreeDataProvider<DependencyItem> {
    private _onDidChangeTreeData: vscode.EventEmitter<DependencyItem | undefined | null | void> =
        new vscode.EventEmitter<DependencyItem | undefined | null | void>();
    readonly onDidChangeTreeData: vscode.Event<DependencyItem | undefined | null | void> =
        this._onDidChangeTreeData.event;

    private dependencies: DependencyNode | null = null;
    private loading = false;

    constructor() {}

    refresh(): void {
        this.dependencies = null;
        this._onDidChangeTreeData.fire();
    }

    getTreeItem(element: DependencyItem): vscode.TreeItem {
        return element;
    }

    async getChildren(element?: DependencyItem): Promise<DependencyItem[]> {
        if (this.loading) {
            return [];
        }

        if (!element) {
            // Root level - fetch dependencies
            if (!this.dependencies) {
                await this.fetchDependencies();
            }

            if (this.dependencies) {
                // Return direct dependencies of root
                return this.getDirectDependencies();
            }

            return [];
        }

        // Return children of the given element
        if (element.node.dependencies && element.node.dependencies.length > 0) {
            return element.node.dependencies.map(dep =>
                new DependencyItem(dep, this.hasChildren(dep))
            );
        }

        return [];
    }

    private getDirectDependencies(): DependencyItem[] {
        if (!this.dependencies || !this.dependencies.dependencies) {
            return [];
        }

        return this.dependencies.dependencies.map(dep =>
            new DependencyItem(dep, this.hasChildren(dep))
        );
    }

    private hasChildren(node: DependencyNode): boolean {
        return node.dependencies !== undefined && node.dependencies.length > 0;
    }

    private async fetchDependencies(): Promise<void> {
        const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
        if (!workspaceFolder) {
            return;
        }

        this.loading = true;

        try {
            const ccgoPath = vscode.workspace.getConfiguration('ccgo').get('executablePath', 'ccgo');
            const result = await this.execCcgoTree(ccgoPath, workspaceFolder.uri.fsPath);

            if (result) {
                this.dependencies = result.root;
            }
        } catch (error) {
            console.error('Failed to fetch dependencies:', error);
            vscode.window.showWarningMessage(`CCGO: Failed to load dependencies. ${error}`);
        } finally {
            this.loading = false;
        }
    }

    private execCcgoTree(ccgoPath: string, cwd: string): Promise<TreeOutput | null> {
        return new Promise((resolve) => {
            cp.exec(`${ccgoPath} tree --format json`, { cwd }, (error, stdout, stderr) => {
                if (error) {
                    console.error('ccgo tree error:', error);
                    console.error('stderr:', stderr);
                    resolve(null);
                    return;
                }

                try {
                    const output = JSON.parse(stdout);
                    resolve(output);
                } catch (parseError) {
                    console.error('Failed to parse ccgo tree output:', parseError);
                    resolve(null);
                }
            });
        });
    }
}

export class DependencyItem extends vscode.TreeItem {
    constructor(
        public readonly node: DependencyNode,
        hasChildren: boolean
    ) {
        super(
            node.name,
            hasChildren
                ? vscode.TreeItemCollapsibleState.Collapsed
                : vscode.TreeItemCollapsibleState.None
        );

        this.description = node.version;
        this.tooltip = this.buildTooltip();
        this.contextValue = 'dependency';
        this.iconPath = this.getIcon();
    }

    private buildTooltip(): string {
        const parts: string[] = [`${this.node.name} v${this.node.version}`];

        if (this.node.source) {
            parts.push(`Source: ${this.node.source}`);
        }

        if (this.node.path) {
            parts.push(`Path: ${this.node.path}`);
        }

        if (this.node.dependencies && this.node.dependencies.length > 0) {
            parts.push(`Dependencies: ${this.node.dependencies.length}`);
        }

        return parts.join('\n');
    }

    private getIcon(): vscode.ThemeIcon {
        if (this.node.source === 'git') {
            return new vscode.ThemeIcon('git-branch');
        } else if (this.node.source === 'path') {
            return new vscode.ThemeIcon('folder');
        } else if (this.node.source === 'registry') {
            return new vscode.ThemeIcon('cloud-download');
        }
        return new vscode.ThemeIcon('package');
    }
}
