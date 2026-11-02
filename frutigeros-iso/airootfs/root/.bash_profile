# FrutigerOS live login shell.
#
# On tty1, automatically start the graphical session once. The session script
# falls back to a login shell when no host display is available, and sets
# FRUTIGEROS_SESSION_STARTED to avoid a restart loop.
if [ -z "${FRUTIGEROS_SESSION_STARTED:-}" ] && [ "$(tty 2>/dev/null)" = "/dev/tty1" ]; then
  export FRUTIGEROS_SESSION_STARTED=1
  if [ -x /usr/local/bin/frutigeros-session ]; then
    exec /usr/local/bin/frutigeros-session
  fi
fi

if [ -f /etc/profile ]; then
  . /etc/profile
fi

if [ -f "$HOME/.bashrc" ]; then
  . "$HOME/.bashrc"
fi
