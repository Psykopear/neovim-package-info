" Initialize the channel
if !exists('s:packageInfoJobId')
    let s:packageInfoJobId = 0
endif

let s:bin = expand('%:p:h/neovim-package-info')

let s:cargoToml = 'cargo-toml'
let s:packageJson = 'package-json'
let s:pipfile = 'pipfile'

" Initialize RPC
function! s:initRpc()
    if s:packageInfoJobId == 0
        let jobid = jobstart([s:bin], { 'rpc': v:true })
        return jobid
    else
        return s:packageInfoJobId
    endif
endfunction

function! s:sendMessage(...)
    " Try sending message, if there is an error it should
    " mean the process crashed for some reason, so we try
    " to bring it back first
    try
        call rpcnotify(s:packageInfoJobId, a:1, a:2, a:3)
    catch /.*/
        let s:packageInfoJobId = 0
        let id = s:initRpc()
        let s:packageInfoJobId = id
        call rpcnotify(s:packageInfoJobId, a:1, a:2, a:3)
    endtry
endfunction

function! s:configureCommands()
  augroup packageInfo
    autocmd!
    autocmd BufEnter *Cargo.toml :call s:sendMessage(s:cargoToml, expand("%:p"), bufnr("%"))
    autocmd BufEnter *package.json :call s:sendMessage(s:packageJson, expand("%:p"), bufnr("%"))
    autocmd BufEnter *Pipfile :call s:sendMessage(s:pipfile, expand("%:p"), bufnr("%"))
  augroup END
endfunction

function! s:connect()
  let id = s:initRpc()

  if 0 == id
    echoerr "neovim-package-info: cannot start rpc process"
  elseif -1 == id
    echoerr "neovim-package-info: rpc process is not executable"
  else
    let s:packageInfoJobId = id
    call s:configureCommands()
  endif
endfunction



call s:connect()

