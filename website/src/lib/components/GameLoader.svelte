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
  });

  onDestroy(() => {
    if (iframeSrc) {
      URL.revokeObjectURL(iframeSrc);
    }
  });
</script>

<iframe 
  src={iframeSrc} 
  title="game-frame"
  class="game-frame"
></iframe>

<style>
  .game-frame {
    width: 100%;
    height: 100%;
    border: none;
    display: block;
  }
</style>
