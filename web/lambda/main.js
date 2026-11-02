const shpCache = {
  files: {},

  // Load a .shp file and cache it
  load: function(filename) {
    return fetch(`./shp/${filename}`)
      .then(response => {
        if (!response.ok) {
          throw new Error(`Failed to load ${filename}`);
        }
        return response.text();
      })
      .then(content => {
        this.files[filename] = content;
        return content;
      })
      .catch(error => {
        console.error(`Error loading ${filename}:`, error);
        throw error;
      });
  },

  // Get a file from cache or load it if not cached
  get: function(filename) {
    if (this.files[filename]) {
      return Promise.resolve(this.files[filename]);
    }
    return this.load(filename);
  }
};

const LOCAL_STORAGE_KEY = "sheepda-editor-state";

// Initialize the application
function initializeApp() {
  // Load prelude first
  shpCache.get('prelude.shp')
    .then(content => {
      document.getElementById("field-stdlib").value = content;

      // After prelude is loaded, check loading priority:
      // 1. URL hash (shared code)
      // 2. Local storage (previously edited code)
      // 3. Default example
      if (location.hash && location.hash !== "#") {
        loadFromHash();
      } else if (hasLocalStorage()) {
        loadFromLocalStorage();
      } else {
        loadDefaultCode();
      }
    })
    .catch(error => {
      document.getElementById("field-stdlib").value = "Error loading prelude file";
    });
}

function run() {
  const runButton = document.getElementById("field-run");
  runButton.disabled = true;
  runButton.textContent = "Running...";

  // Save current state to local storage before running
  saveToLocalStorage();

  setTimeout(function() {
    const code = document.getElementById("field-code").value;
    const stdlibChecked = document.getElementById("field-stdlib-checkbox").checked;
    const outputType = document.getElementById("field-output-type").value;

    const preludeContent = shpCache.files['prelude.shp'] || '';
    const codeToRun = stdlibChecked ? preludeContent + code : code;

    try {
      const rv = sheepda.eval(codeToRun, outputType);
      if (rv[1] != "") {
        document.getElementById("field-output").value = "Error: " + rv[1].trim();
      } else {
        document.getElementById("field-output").value = rv[0].trim();
      }
    } catch(err) {
      document.getElementById("field-output").value = "Error: " + err.toString().trim();
    }

    runButton.textContent = "Run";
    runButton.disabled = false;
  }, 100);
}

function saveToHash() {
  const stateObject = getCurrentState();

  // Create hash and update URL without reloading the page
  const hashValue = btoa(encodeURIComponent(JSON.stringify(stateObject)));
  window.history.replaceState(null, "", "#" + hashValue);

  // Also save to local storage when sharing
  saveToLocalStorage();
}

function saveToLocalStorage() {
  try {
    const stateObject = getCurrentState();
    localStorage.setItem(LOCAL_STORAGE_KEY, JSON.stringify(stateObject));
  } catch (error) {
    console.error("Error saving to local storage:", error);
  }
}

function getCurrentState() {
  return {
    stdlib: document.getElementById("field-stdlib-checkbox").checked,
    code: document.getElementById("field-code").value
  };
}

function hasLocalStorage() {
  try {
    return localStorage.getItem(LOCAL_STORAGE_KEY) !== null;
  } catch (error) {
    console.error("Error accessing local storage:", error);
    return false;
  }
}

function loadFromLocalStorage() {
  try {
    const stateJSON = localStorage.getItem(LOCAL_STORAGE_KEY);
    if (stateJSON) {
      const stateObject = JSON.parse(stateJSON);
      applyState(stateObject);
      console.log("Loaded from local storage");
      return true;
    }
  } catch (error) {
    console.error("Error loading from local storage:", error);
  }

  // Fallback to default if local storage loading fails
  loadDefaultCode();
  return false;
}

function loadFromHash() {
  if (location.hash && location.hash !== "#") {
    try {
      const hashValue = location.hash.slice(1); // Remove the # symbol
      const stateObject = JSON.parse(decodeURIComponent(atob(hashValue)));
      applyState(stateObject);

      // Also save hash state to local storage
      saveToLocalStorage();
      return true;
    } catch (error) {
      console.error("Error parsing hash:", error);
    }
  }

  // Fallback to local storage if hash loading fails
  if (hasLocalStorage()) {
    return loadFromLocalStorage();
  } else {
    // Fallback to default if both hash and local storage fail
    loadDefaultCode();
    return false;
  }
}

function applyState(stateObject) {
  document.getElementById("field-stdlib-checkbox").checked = stateObject.stdlib;
  document.getElementById("field-code").value = stateObject.code;
}

function loadDefaultCode() {
  const codeEditor = document.getElementById("field-code");
  codeEditor.value = "Loading default example...";

  shpCache.get('default.shp')
    .then(content => {
      codeEditor.value = content;
      // Also save default code to local storage for future visits
      saveToLocalStorage();
    })
    .catch(error => {
      codeEditor.value = "Error loading default example";
    });
}

// Initialize when DOM is fully loaded
document.addEventListener('DOMContentLoaded', function() {
  // Set up hash change listener
  window.addEventListener('hashchange', loadFromHash);

  // Add event listeners for buttons
  document.getElementById("field-run").addEventListener("click", function(e) {
    e.preventDefault();
    run();
  });

  document.getElementById("field-share").addEventListener("click", function(e) {
    e.preventDefault();
    saveToHash();
  });

  // Initialize the app
  initializeApp();
});
