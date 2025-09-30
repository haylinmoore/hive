{
  config,
  lib,
  pkgs,
  ...
}:

{
  programs.neovim = {
    enable = true;
    extraPackages = with pkgs; [
      # LazyVim
      lua-language-server
      stylua
      # Telescope
      ripgrep
      # Rust
      rust-analyzer
    ];

    plugins = with pkgs.vimPlugins; [
      lazy-nvim
    ];

    extraLuaConfig =
      let
        plugins = with pkgs.vimPlugins; [
          # LazyVim
          LazyVim
          bufferline-nvim
          cmp-buffer
          cmp-nvim-lsp
          cmp-path
          cmp_luasnip
          dashboard-nvim
          dressing-nvim
          flash-nvim
          friendly-snippets
          gitsigns-nvim
          indent-blankline-nvim
          lualine-nvim
          neo-tree-nvim
          neoconf-nvim
          neodev-nvim
          noice-nvim
          nui-nvim
          nvim-cmp
          nvim-lint
          nvim-lspconfig
          nvim-notify
          nvim-spectre
          nvim-treesitter
          nvim-treesitter-context
          nvim-treesitter-textobjects
          nvim-ts-autotag
          nvim-ts-context-commentstring
          nvim-web-devicons
          persistence-nvim
          plenary-nvim
          telescope-fzf-native-nvim
          telescope-nvim
          telescope-live-grep-args-nvim
          todo-comments-nvim
          tokyonight-nvim
          trouble-nvim
          vim-illuminate
          vim-startuptime
          which-key-nvim
          {
            name = "LuaSnip";
            path = luasnip;
          }
          {
            name = "catppuccin";
            path = catppuccin-nvim;
          }
          {
            name = "mini.ai";
            path = mini-nvim;
          }
          {
            name = "mini.bufremove";
            path = mini-nvim;
          }
          {
            name = "mini.comment";
            path = mini-nvim;
          }
          {
            name = "mini.indentscope";
            path = mini-nvim;
          }
          {
            name = "mini.pairs";
            path = mini-nvim;
          }
          {
            name = "mini.surround";
            path = mini-nvim;
          }
          # Rust support
          rustaceanvim
          crates-nvim
        ];
        mkEntryFromDrv =
          drv:
          if lib.isDerivation drv then
            {
              name = "${lib.getName drv}";
              path = drv;
            }
          else
            drv;
        lazyPath = pkgs.linkFarm "lazy-plugins" (builtins.map mkEntryFromDrv plugins);
      in
      ''
        -- Enable true color support
        vim.opt.termguicolors = true

        require("lazy").setup({
          defaults = {
            lazy = true,
          },
          dev = {
            -- reuse files from pkgs.vimPlugins.*
            path = "${lazyPath}",
            patterns = { "" },
            -- fallback to download
            fallback = true,
          },
          spec = {
            { "LazyVim/LazyVim", import = "lazyvim.plugins" },
            -- Import LazyVim LSP extras for proper keybindings and hover
            { import = "lazyvim.plugins.extras.lsp.none-ls" },
            { import = "lazyvim.plugins.extras.lang.rust" },
            -- The following configs are needed for fixing lazyvim on nix
            -- force enable telescope-fzf-native.nvim
            { "nvim-telescope/telescope-fzf-native.nvim", enabled = true },
            -- telescope live grep args
            { 
              "nvim-telescope/telescope-live-grep-args.nvim",
              dependencies = { "nvim-telescope/telescope.nvim" },
              config = function()
                require("telescope").load_extension("live_grep_args")
              end,
              keys = {
                {
                  "<leader>fs",
                  function()
                    require("telescope").extensions.live_grep_args.live_grep_args()
                  end,
                  desc = "Live grep with args",
                },
              },
            },
            -- disable mason.nvim, use programs.neovim.extraPackages
            { "williamboman/mason-lspconfig.nvim", enabled = false },
            { "williamboman/mason.nvim", enabled = false },
            -- import/override with your plugins
            -- { import = "plugins" },
            -- Configure rust-analyzer with proper hover support
            {
              "neovim/nvim-lspconfig",
              opts = {
                servers = {
                  rust_analyzer = {
                    cmd = { "rust-analyzer" },
                    settings = {
                      ["rust-analyzer"] = {
                        cargo = {
                          allFeatures = true,
                        },
                        checkOnSave = {
                          command = "clippy",
                        },
                        diagnostics = {
                          enable = true,
                        },
                      },
                    },
                  },
                },
              },
            },
            -- treesitter handled by xdg.configFile."nvim/parser", put this line at the end of spec to clear ensure_installed
            { 
              "nvim-treesitter/nvim-treesitter", 
              opts = function(_, opts)
                opts.ensure_installed = {}
                opts.auto_install = false
                -- Completely disable the install command
                require("nvim-treesitter.install").commands = {}
                return opts
              end
            },
          },
        })

        -- Configure native OSC52 clipboard for SSH
        vim.g.clipboard = {
          name = 'OSC 52',
          copy = {
            ['+'] = require('vim.ui.clipboard.osc52').copy('+'),
            ['*'] = require('vim.ui.clipboard.osc52').copy('*'),
          },
          paste = {
            ['+'] = require('vim.ui.clipboard.osc52').paste('+'),
            ['*'] = require('vim.ui.clipboard.osc52').paste('*'),
          },
        }

        vim.opt.clipboard = 'unnamedplus'

        -- Claude floating window functionality
        local function create_floating_window()
          local width = math.floor(vim.o.columns * 0.8)
          local height = math.floor(vim.o.lines * 0.8)
          
          local row = math.floor((vim.o.lines - height) / 2)
          local col = math.floor((vim.o.columns - width) / 2)
          
          local buf = vim.api.nvim_create_buf(false, true)
          
          local win = vim.api.nvim_open_win(buf, true, {
            relative = 'editor',
            width = width,
            height = height,
            row = row,
            col = col,
            style = 'minimal',
            border = 'rounded',
            title = ' Claude Code ',
            title_pos = 'center',
          })
          
          return buf, win
        end

        local claude_buf = nil
        local claude_win = nil

        local function toggle_claude()
          if claude_win and vim.api.nvim_win_is_valid(claude_win) then
            -- Just hide the window, keep the buffer
            vim.api.nvim_win_close(claude_win, true)
            claude_win = nil
          else
            -- Create or reuse existing buffer
            if not claude_buf or not vim.api.nvim_buf_is_valid(claude_buf) then
              claude_buf, claude_win = create_floating_window()
              
              vim.fn.termopen('claude', {
                on_exit = function()
                  -- Only clean up when Claude actually exits
                  claude_buf = nil
                  claude_win = nil
                end
              })
            else
              -- Reuse existing buffer with persistent Claude session
              local _, win = create_floating_window()
              vim.api.nvim_win_set_buf(win, claude_buf)
              claude_win = win
            end
            
            vim.cmd('startinsert')
          end
        end

        vim.keymap.set('n', '<leader>cc', toggle_claude, { desc = 'Toggle Claude floating window' })
        vim.keymap.set({'n', 'i', 'v', 't'}, '<C-\\><C-\\>', toggle_claude, { desc = 'Toggle Claude floating window (all modes)' })

        -- Terminal mode keybind to exit terminal mode easily
        vim.keymap.set('t', '<C-\\><C-n>', '<C-\\><C-n>', { desc = 'Exit terminal mode' })
      '';
  };

  # https://github.com/nvim-treesitter/nvim-treesitter#i-get-query-error-invalid-node-type-at-position
  xdg.configFile."nvim/parser".source =
    let
      parsers = pkgs.symlinkJoin {
        name = "treesitter-parsers";
        paths =
          (pkgs.vimPlugins.nvim-treesitter.withPlugins (
            plugins: with plugins; [
              c
              lua
            ]
          )).dependencies;
      };
    in
    "${parsers}/parser";

}
