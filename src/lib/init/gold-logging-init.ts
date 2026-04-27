import { invoke } from "@tauri-apps/api/core";
import { RAIDS } from "../data/raids";

/**
 * Initialize gold logging system with frontend raid data
 * This ensures the backend has access to raid gold values without hardcoding
 */
export async function initializeGoldLogging() {
    try {
        // Only initialize raid data state - NO automatic gold processing
        console.log("Initializing gold logging system...");
        await invoke("update_raid_data_state", { raidsData: RAIDS });
        console.log("Raid data state updated successfully");
        
        // Test manual gold processing after a short delay
        setTimeout(async () => {
            try {
                console.log("Testing manual gold processing...");
                const result = await processGoldLogs();
                console.log("Test gold processing completed:", result);
            } catch (error) {
                console.error("Test gold processing failed:", error);
            }
        }, 2000); // 2 second delay
        
    } catch (error) {
        console.error("Failed to initialize gold logging:", error);
    }
}

// Manual gold processing function - can be called when needed
export async function processGoldLogs() {
    try {
        console.log("Processing gold logs...");
        const result = await invoke("trigger_gold_processing");
        console.log("Gold processing result:", result);
        return result;
    } catch (error) {
        console.error("Failed to process gold logs:", error);
        throw error;
    }
}
