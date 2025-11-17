" Enable filetype detection, plugins, and indent
filetype plugin indent on
syntax enable

set number
set norelativenumber
set mouse=a
set nohlsearch
set shiftwidth=4
set tabstop=4
set autoindent
set smartindent
set smartcase
set ff=unix
set spell                  " Use spell check
set spelllang=en_us        " Use US English for spell check
set splitbelow             " Split horizontal windows below of current
set splitright             " Split vertical windows right of current
set synmaxcol=300          " Look only to column 300 to decide syntax

" Just use VCS
set nobackup
set noswapfile

" Auto-reload files when changed externally
set autoread
augroup auto_read
  autocmd!
  autocmd FocusGained,BufEnter,CursorHold,CursorHoldI * checktime
augroup END

" Tab shortcuts
nnoremap <C-j> :tabprevious<CR>
nnoremap <C-k> :tabnext<CR>

" Better indenting - keep selection after indent
vnoremap < <gv
vnoremap > >gv

" Tag navigation
nnoremap <C-S-T> :tnext<CR>

" * stays on current word
:nnoremap * *N
