import * as vscode from 'vscode';

interface CcgoTaskDefinition extends vscode.TaskDefinition {
    command: string;
    platform?: string;
    architectures?: string[];
    release?: boolean;
    linkType?: 'static' | 'shared' | 'both';
}

export class CcgoTaskProvider implements vscode.TaskProvider {
    static CcgoType = 'ccgo';

    private tasks: vscode.Task[] | undefined;

    constructor() {}

    async provideTasks(): Promise<vscode.Task[]> {
        return this.getTasks();
    }

    resolveTask(task: vscode.Task): vscode.Task | undefined {
        const definition = task.definition as CcgoTaskDefinition;

        if (definition.command) {
            return this.createTask(definition);
        }

        return undefined;
    }

    private getTasks(): vscode.Task[] {
        if (this.tasks !== undefined) {
            return this.tasks;
        }

        this.tasks = [];
        const platforms = ['android', 'ios', 'macos', 'linux', 'windows', 'ohos', 'kmp'];

        // Build tasks for each platform
        for (const platform of platforms) {
            // Debug build
            this.tasks.push(this.createTask({
                type: CcgoTaskProvider.CcgoType,
                command: 'build',
                platform,
                release: false
            }));

            // Release build
            this.tasks.push(this.createTask({
                type: CcgoTaskProvider.CcgoType,
                command: 'build',
                platform,
                release: true
            }));
        }

        // Common tasks
        this.tasks.push(this.createTask({
            type: CcgoTaskProvider.CcgoType,
            command: 'test'
        }));

        this.tasks.push(this.createTask({
            type: CcgoTaskProvider.CcgoType,
            command: 'bench',
            release: true
        }));

        this.tasks.push(this.createTask({
            type: CcgoTaskProvider.CcgoType,
            command: 'install'
        }));

        this.tasks.push(this.createTask({
            type: CcgoTaskProvider.CcgoType,
            command: 'clean'
        }));

        this.tasks.push(this.createTask({
            type: CcgoTaskProvider.CcgoType,
            command: 'doc'
        }));

        return this.tasks;
    }

    private createTask(definition: CcgoTaskDefinition): vscode.Task {
        const ccgoPath = vscode.workspace.getConfiguration('ccgo').get('executablePath', 'ccgo');
        const args = this.buildArgs(definition);
        const taskName = this.getTaskName(definition);

        const execution = new vscode.ShellExecution(ccgoPath, args);

        const task = new vscode.Task(
            definition,
            vscode.TaskScope.Workspace,
            taskName,
            'ccgo',
            execution,
            '$gcc' // Use GCC problem matcher for C++ build output
        );

        task.group = this.getTaskGroup(definition.command);
        task.presentationOptions = {
            reveal: vscode.TaskRevealKind.Always,
            panel: vscode.TaskPanelKind.Shared
        };

        return task;
    }

    private buildArgs(definition: CcgoTaskDefinition): string[] {
        const args: string[] = [definition.command];

        if (definition.platform) {
            args.push(definition.platform);
        }

        if (definition.release) {
            args.push('--release');
        }

        if (definition.architectures && definition.architectures.length > 0) {
            args.push('--arch', definition.architectures.join(','));
        }

        if (definition.linkType) {
            args.push('--link-type', definition.linkType);
        }

        return args;
    }

    private getTaskName(definition: CcgoTaskDefinition): string {
        const parts: string[] = [];

        if (definition.platform) {
            parts.push(definition.platform);
        }

        parts.push(definition.command);

        if (definition.release) {
            parts.push('(release)');
        } else if (definition.command === 'build') {
            parts.push('(debug)');
        }

        return parts.join(' ');
    }

    private getTaskGroup(command: string): vscode.TaskGroup | undefined {
        switch (command) {
            case 'build':
                return vscode.TaskGroup.Build;
            case 'test':
            case 'bench':
                return vscode.TaskGroup.Test;
            case 'clean':
                return vscode.TaskGroup.Clean;
            default:
                return undefined;
        }
    }
}
