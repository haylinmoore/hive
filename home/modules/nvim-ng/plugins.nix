{ config, pkgs, ... }:
{
  programs.neovim.plugins = with pkgs.vimPlugins; [
    nerdtree
    vim-sleuth
    {
      plugin = nvim-unception;
      type = "lua";
      config = ''
        -- Disable nvim nesting, use parent nvim for git commits, etc.
        vim.g.unception_block_while_host_edits = true
      '';
    }
    {
      plugin = catppuccin-nvim;
      type = "lua";
      config = ''
        -- Enable termguicolors for true color support in tmux
        vim.opt.termguicolors = true
        vim.opt.background = 'dark'

        -- Setup catppuccin
        require("catppuccin").setup({
          flavour = "mocha", -- latte, frappe, macchiato, mocha
          transparent_background = false,
          term_colors = true,
          styles = {
            comments = { },  -- Disable italic for comments only
          },
        })

        -- Set colorscheme
        vim.cmd.colorscheme("catppuccin")
      '';
    }
    {
      plugin = fzf-vim;
      config = ''
        nnoremap <C-p> :Files<CR>
        " Rg in current file's directory
        nnoremap <C-S-p> :call fzf#vim#grep('rg --column --line-number --no-heading --color=always --smart-case -- ', 1, fzf#vim#with_preview({'dir': expand('%:h')}), 0)<CR>
        " Search open buffers
        nnoremap <C-S-b> :Buffers<CR>
      '';
    }
    {
      plugin = lualine-nvim;
      type = "lua";
      config = ''
        require('lualine').setup {
          options = { theme = 'catppuccin' },
          sections = {
            lualine_c = {
              {
                'filename',
                path = 1
              }
            }
          }
        }
      '';
    }
    {
      plugin = vim-oscyank;
      config = "autocmd TextYankPost * if v:event.operator is 'y' && v:event.regname is '' | execute 'OSCYankRegister \"' | endif";
    }
    {
      plugin = nvim-lspconfig;
      type = "lua";
      config = ''
        local lspconfig = require('lspconfig')

        -- Rust analyzer configuration for Qumulo
        lspconfig.rust_analyzer.setup({
          cmd = { "/opt/qumulo/toolchain/bin/rust-analyzer" },
          settings = {
            ["rust-analyzer"] = {
              rustfmt = {
                overrideCommand = { "/opt/qumulo/toolchain/bin/rustfmt" }
              },
              server = {
                extraEnv = {
                  CHALK_OVERFLOW_DEPTH = "500"
                }
              }
            }
          }
        })

        -- LSP keybindings (only apply when LSP is attached)
        vim.api.nvim_create_autocmd('LspAttach', {
          group = vim.api.nvim_create_augroup('UserLspConfig', {}),
          callback = function(ev)
            local opts = { buffer = ev.buf }
            vim.keymap.set('n', 'gd', vim.lsp.buf.definition, opts)
            vim.keymap.set('n', 'K', vim.lsp.buf.hover, opts)
            vim.keymap.set('n', 'gi', vim.lsp.buf.implementation, opts)
            vim.keymap.set('n', 'gr', vim.lsp.buf.references, opts)
            vim.keymap.set('n', '<leader>rn', vim.lsp.buf.rename, opts)
            vim.keymap.set('n', '[g', vim.diagnostic.goto_prev, opts)
            vim.keymap.set('n', ']g', vim.diagnostic.goto_next, opts)
          end,
        })
      '';
    }
    {
      plugin = pkgs.vimUtils.buildVimPlugin {
        name = "terminal-toggle";
        src = pkgs.writeTextDir "plugin/terminal-toggle.lua" ''
          -- Terminal toggle state
          local term_buf = nil
          local term_win = nil

          local function toggle_terminal()
            -- If terminal window is open, close it
            if term_win and vim.api.nvim_win_is_valid(term_win) then
              vim.api.nvim_win_close(term_win, true)
              term_win = nil
              return
            end

            -- Create a new terminal buffer if it doesn't exist or is invalid
            if not term_buf or not vim.api.nvim_buf_is_valid(term_buf) then
              term_buf = vim.api.nvim_create_buf(false, true)
              vim.api.nvim_buf_set_option(term_buf, 'bufhidden', 'hide')
            end

            -- Create a split window at the bottom with 15 rows
            vim.cmd('botright 15split')
            term_win = vim.api.nvim_get_current_win()
            vim.api.nvim_win_set_buf(term_win, term_buf)

            -- Start terminal if buffer is empty (new buffer)
            if vim.api.nvim_buf_line_count(term_buf) == 1 and vim.api.nvim_buf_get_lines(term_buf, 0, 1, false)[1] == "" then
              vim.fn.termopen(vim.o.shell)
            end

            -- Enter insert mode in terminal
            vim.cmd('startinsert')
          end

          -- Set up the keybinding for Ctrl-/
          -- Note: Ctrl-/ is represented as <C-_> in terminal vim
          vim.keymap.set({'n', 't'}, '<C-_>', toggle_terminal, { noremap = true, silent = true })
        '';
      };
      type = "lua";
    }
  ];
}
