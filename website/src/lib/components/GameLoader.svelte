<script lang="ts">
  import { onMount, onDestroy } from 'svelte';

  interface Props {
      version: string;
      name: string;
      lobby?: string | null;
      lobby_size?: string | null;
      matchbox?: string | null;
  }

  let { 
    version, 
    name, 
    lobby = null, 
    lobby_size = null, 
    matchbox = null 
  }: Props = $props();

  let iframeSrc: string = $state('');
  let container: HTMLDivElement | undefined = $state();
  let isFullscreen = $state(false);
  
  onMount(() => {
    const modulePath = `/${version}/${name}/wasm.js`;
    
    // Construct the HTML content for the isolated iframe
    const htmlContent = `
<!DOCTYPE html>
<html>
<head>
  <meta charset="UTF-8" />
  <style>
    * {
      font-family: 'Source Code Pro', monospace;
      color: #777;
    }
    body, canvas {
      margin: 0px;
      width: 100%;
      height: 100%;
      overflow: hidden;
      background-attachment: fixed;
      background-color: oklch(40.91% 0 none);
    }
    canvas {
      display: block;
      outline: none;
    }
  </style>
</head>
<body>
    <canvas id="bevy-canvas" tabIndex="0" autofocus></canvas>
    <script type="module">
      // 1. Define global helper expected by WASM
      window.download_log_file_js = (filename, content) => {
          const blob = new Blob([content], { type: 'text/plain;charset=utf-8' });
          const link = document.createElement('a');
          const url = URL.createObjectURL(blob);
          link.href = url;
          link.download = filename;
          document.body.appendChild(link);
          link.click();
          document.body.removeChild(link);
          URL.revokeObjectURL(url);
          console.log("Log file download initiated: " + filename);
      };

      // 2. Configure canvas attributes
      const canvas = document.getElementById("bevy-canvas");
      const matchbox = "${matchbox || ''}";
      const lobby = "${lobby || ''}";
      const lobby_size = "${lobby_size || '2'}";

      if (matchbox && matchbox.length > 0) {
        canvas.setAttribute("data-matchbox", matchbox);
        canvas.setAttribute("data-lobby", lobby || "test");
        canvas.setAttribute("data-number-player", lobby_size);
        console.log("MATCHBOX " + matchbox + " NUMBER " + lobby_size + " LOBBY " + lobby);
      }

      function auto_focus() {
        if (!lobby && !matchbox) {
           console.warn("No lobby provided!");
        }
        canvas.focus();
      }

      // 3. Load WASM
      import init from "${modulePath}";

      init().then(
        () => {
          console.log("WASM init success");
          auto_focus();
        },
        (e) => console.error("Error init WASM: ", e)
      ).catch((e) => {
          console.error("Error executing WASM module: ", e)
      });
    <\/script>
</body>
</html>
    `;

    const blob = new Blob([htmlContent], { type: 'text/html' });
    iframeSrc = URL.createObjectURL(blob);

    document.addEventListener('fullscreenchange', handleFullscreenChange);
    document.addEventListener('webkitfullscreenchange', handleFullscreenChange);
  });

  onDestroy(() => {
    if (iframeSrc) {
      URL.revokeObjectURL(iframeSrc);
    }
    if (typeof document !== 'undefined') {
        document.removeEventListener('fullscreenchange', handleFullscreenChange);
        document.removeEventListener('webkitfullscreenchange', handleFullscreenChange);
    }
  });

  function handleFullscreenChange() {
      isFullscreen = !!document.fullscreenElement;
  }

  async function requestFullscreen() {
      if (!container) return;
      try {
          if (container.requestFullscreen) {
              await container.requestFullscreen();
          } else if ((container as any).webkitRequestFullscreen) {
              await (container as any).webkitRequestFullscreen();
          }
          
          if (screen.orientation && 'lock' in screen.orientation) {
              // @ts-ignore
              await screen.orientation.lock('landscape').catch((e) => console.warn('Orientation lock failed', e));
          }
          isFullscreen = true;
      } catch (err) {
          console.error("Fullscreen failed", err);
          isFullscreen = true; // Allow playing even if FS fails
      }
  }
</script>

<div class="relative w-full h-full bg-black" bind:this={container}>
    {#if !isFullscreen}
        <button 
            class="absolute bottom-4 right-4 btn-icon variant-filled-primary shadow-lg z-50 opacity-70 hover:opacity-100 transition-opacity" 
            onclick={requestFullscreen}
            aria-label="Enter Fullscreen"
        >
            <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M8 3H5a2 2 0 0 0-2 2v3"/><path d="M21 8V5a2 2 0 0 0-2-2h-3"/><path d="M3 16v3a2 2 0 0 0 2 2h3"/><path d="M16 21h3a2 2 0 0 0 2-2v-3"/></svg>
        </button>
    {/if}

    <iframe 
      src={iframeSrc} 
      title="game-frame"
      class="w-full h-full border-none block"
      allow="autoplay; fullscreen"
      allowfullscreen
    ></iframe>
</div>
