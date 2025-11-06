" Work-specific vim configuration

" Work-specific settings
set tags=./tags;,tags;     " Look for tags in cwd and search upward
set textwidth=80
set colorcolumn=+1  " Set Color Column at textwidth + 1.

" Tag navigation
nnoremap <C-S-T> :tnext<CR>

autocmd FileType c setlocal textwidth=100
autocmd FileType html setlocal textwidth=120
autocmd FileType javascript setlocal textwidth=120
autocmd FileType typescript setlocal textwidth=120
autocmd FileType typescriptreact setlocal textwidth=120
autocmd FileType python setlocal textwidth=100
autocmd FileType rust setlocal textwidth=100
autocmd BufRead,BufNewFile *.x setlocal textwidth=100

" Toggle between test files
function! GoToTest()
    let newfile = expand('%:r') . '_test.' . expand('%:e')
    execute "e " . fnameescape(newfile)
endfunction

function! GoFromTest()
    let newfile = substitute(expand('%:r'), '_test$', '', '') . '.' . expand('%:e')
    execute "e " . fnameescape(newfile)
endfunction

function! ToggleTestFile()
    " If in abc_test.xyz file, open abc.xyz
    " If in abc.xyz, open abc_test.xyz
    let base = expand('%:r')
    if base =~ '.*_test'
        call GoFromTest()
    else
        call GoToTest()
    endif
endfunction

" Toggle between header and source files
function! GoToHeader()
    let newfile = expand('%:r') . '.h'
    execute "e " . fnameescape(newfile)
endfunction

function! GoToSource()
    let newfile = expand('%:r') . '.c'
    execute "e " . fnameescape(newfile)
endfunction

function! ToggleHeaderSource()
    " If in abc.c file, open abc.h
    " If in abc.h, open abc.c
    let ext = expand('%:e')
    if ext =~ 'c'
        call GoToHeader()
    else
        call GoToSource()
    endif
endfunction

" Toggle between source file and generated build file
function! ToggleGen()
    let current = expand('%')
    if current =~ 'gen' || current =~ 'build/debug/'
        " Drop leading build/debug/
        let original = substitute(current, 'build/debug/', '', '')
        " Strip `.gen` and everything after it
        let original = substitute(original, '\.gen.*$', '', '')
        execute "e " . fnameescape(original)
    else
        " Otherwise, open the generated file
        let newfile = 'build/debug/' . current . '.gen.h'
        execute "e " . fnameescape(newfile)
    endif
endfunction

" File toggle commands
command! T call ToggleTestFile()
command! H call ToggleHeaderSource()
command! C call ToggleHeaderSource()
command! G call ToggleGen()

" Keybinds for file toggles
nnoremap \G :call ToggleGen()<CR>
nnoremap \H :call ToggleHeaderSource()<CR>

" Build commands
" Async build function using terminal
function! AsyncBuild(...)
    let target = join(a:000, ' ')
    let cmd = 'build --linking-cache rw --linking-cache-directory $PWD/build/.qonstruct/cache/ ' . target

    " Open split at bottom and run build in terminal
    botright 15split
    call termopen(cmd)
    " Return to previous window
    wincmd p
endfunction

" Quick build command - :Build simnode
command! -nargs=* Build call AsyncBuild(<f-args>)

" Quick keybind to prompt for build target
nnoremap <leader>b :Build<Space>

" Toggle quickfix window (open/close)
function! ToggleQuickfix()
    let qf_open = len(filter(getwininfo(), 'v:val.quickfix')) > 0
    if qf_open
        cclose
    else
        copen
    endif
endfunction
nnoremap <leader>q :call ToggleQuickfix()<CR>

" Quickfix buffer settings and keybinds
autocmd FileType qf nnoremap <buffer> q :cclose<CR>
autocmd FileType qf nnoremap <buffer> <leader>q :call ToggleQuickfix()<CR>
autocmd FileType qf setlocal nobuflisted  " Don't list in buffer list
