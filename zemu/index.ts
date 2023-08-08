import process from 'process';

import Zemu, { DEFAULT_START_OPTIONS } from "@zondax/zemu"
import { TModel } from '@zondax/zemu/dist/types';

// Function to read the first CLI argument or use a default value
function readArgOrDefault(idx: number, defaultValue: string): string {
    // Get command line arguments from process.argv array
    const args: string[] = process.argv.slice(2 + idx);

    // Get the first argument if available, otherwise use the default value
    return args.length > 0 ? args[0] : defaultValue;
}

const APP_SEED = 'equip will roof matter pink blind book anxiety banner elbow sun young'

// Async main program
async function main() {

    // Read the first CLI argument or use the default value
    const app_path = readArgOrDefault(0, "app.elf");
    const model = readArgOrDefault(1, "nanosp") as TModel;

    const apiPort = parseInt(readArgOrDefault(2, "8080"));

    await Zemu.checkAndPullImage();
    const sim = new Zemu(app_path, {}, undefined, undefined, apiPort);
    await sim.start({
        ...DEFAULT_START_OPTIONS, model,
        logging: true,
        custom: `-s "${APP_SEED}"`,
    });

    process.on('SIGINT', () => {
        console.log("Received SIGINT, closing...");
        Zemu.stopAllEmuContainers();
        process.exit(0);
    })

    await new Promise<void>(() => {
        while (true) {
            // Empty loop
        }
    });
}

// Call the async main program
main().catch(error => {
    console.error('An error occurred:', error);
});
