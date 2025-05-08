document.addEventListener('DOMContentLoaded', function() {
    const urlParams = new URLSearchParams(window.location.search);
    const matchbox = urlParams.get("matchbox");
    const lobby_size = urlParams.get("lobby_size");
    const lobby = urlParams.get("lobby");
    const name = "./" + urlParams.get("name") + "/wasm.js";

    if (matchbox && matchbox.length > 0) {
      let canvas = document.getElementById("bevy-canvas");
      canvas.setAttribute("data-matchbox", matchbox);
      canvas.setAttribute("data-number-player", lobby_size || 2);

      console.log("MATCHBOX " + matchbox + " NUMBER " + lobby_size + " LOBBY " + lobby);
    }

    import(name).then((module) => {
      console.log(module);
      module.default();
      auto_focus();
    });

    function auto_focus() {
      let canvas = document.getElementsByTagName("bevy-canvas");

      if (!lobby) {
        alert("You failed to provide a lobby , reload the page with ?lobby=mylobbyname as arguments after the path");
      }

      if (!canvas.length) {
        setTimeout(auto_focus, 100);
      } else {
        canvas[0].focus();
      }
    }
});