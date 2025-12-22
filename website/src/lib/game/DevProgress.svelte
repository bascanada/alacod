<script lang="ts">
    // Props for the component
    export let currentVersion: string = import.meta.env.VITE_APP_VERSION || "0.2.0";
    export let releaseVersion: string = '1.0.0';
    export let milestones: Array<{version: string, label: string, complete: boolean}> = [
        { version: '0.0.1', label: 'Initial Setup', complete: true },
        { version: '0.1.0', label: 'Basic Map Generatiom', complete: false },
        { version: '0.2.0', label: 'Character, Weapons and Interaction', complete: false },
        { version: '0.3.0', label: 'Gamemode', complete: false },
        { version: '0.4.0', label: 'Alpha Release', complete: false },
        { version: '0.8.0', label: 'Beta Release', complete: false },
        { version: '1.0.0', label: 'Official Release', complete: false }
    ];
    
    $: progressPercentage = calculateProgress(currentVersion, releaseVersion);
    
    function calculateProgress(current: string, target: string): number {
        let currentSanitize = current.split("-")[0];
        const currentParts = currentSanitize.split('.').map(Number);
        const targetParts = target.split('.').map(Number);
        
        // Simple calculation based on major.minor.patch
        const currentValue = currentParts[0] * 100 + currentParts[1] * 10 + currentParts[2];
        const targetValue = targetParts[0] * 100 + targetParts[1] * 10 + targetParts[2];
        
        return Math.min(100, Math.max(0, (currentValue / targetValue) * 100));
    }
    
    $: {
        milestones = milestones.map(milestone => {
            const milestoneParts = milestone.version.split('.').map(Number);
            const currentParts = currentVersion.split('.').map(Number);
            
            // Compare version numbers
            const isComplete = 
                milestoneParts[0] < currentParts[0] || 
                (milestoneParts[0] === currentParts[0] && milestoneParts[1] < currentParts[1]) ||
                (milestoneParts[0] === currentParts[0] && milestoneParts[1] === currentParts[1] && milestoneParts[2] <= currentParts[2]);
            
            return { ...milestone, complete: isComplete };
        });
    }
</script>


<a href="https://github.com/bascanada/alacod/issues" target="_blank" class="card preset-filled-surface-100-900 border-[1px] border-surface-200-800 card-hover divide-surface-200-800 block max-w-md divide-y overflow-hidden">
    <header>
        <h3 class="h3 m-5 text-left">Project Progress</h3>
    </header>
    
    <article class="space-y-4 p-4">
        <div class="progress-container flex flex-col gap-2">
            <div class="flex justify-between items-center">
                <span class="text-sm opacity-60">Current: {currentVersion}</span>
                <span class="text-sm opacity-60">Target: {releaseVersion}</span>
            </div>
            
            <div class="progress-bar w-full h-4 bg-surface-300-600-token rounded-full relative">
                <div 
                    class="progress-fill h-full rounded-full variant-filled-primary" 
                    style="width: {progressPercentage}%;"
                ></div>
            </div>
            
            <div class="milestones-list">
                <ul class="space-y-2 mt-2">
                    {#each milestones as milestone}
                        <li class="flex items-center gap-2">
                            <div class="w-3 h-3 rounded-full {milestone.complete ? 'variant-filled-success' : 'variant-filled-surface'}"></div>
                            <span class="text-sm {milestone.complete ? 'line-through decoration-2 opacity-70' : ''}">{milestone.version} - {milestone.label}</span>
                        </li>
                    {/each}
                </ul>
            </div>
        </div>
    </article>
    
    <footer class="flex items-center justify-between gap-4 p-4">
        <small class="badge preset-tonal-secondary">Follow the current progress or contribute !</small>
    </footer>
</a>

<style>
    .progress-fill {
        transition: width 0.5s ease-in-out;
    }
  /* Ensure consistent heights */
  .card {
    width: 100%;
  }
  .card article {
    display: flex;
    flex-direction: column;
    min-height: 200px;
  }
  
  /* Optional: Add hover effect */
  .card:hover {
    transform: translateY(-4px);
    transition: transform 0.2s ease-in-out;
  }
</style>