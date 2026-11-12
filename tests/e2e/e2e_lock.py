"""Le verrou qui sépare la remise à zéro de la base e2e des sessions pytest.

Carte 641. `make e2e` vide la base avant chaque suite complète
(`scripts/reset_e2e_db.sh`). Tous les worktrees partagent cette base et son
serveur : vider la base sous une session pytest lancée ailleurs casserait cette
session au milieu, avec des échecs qui n'accuseraient rien de précis.

- Chaque session pytest prend le verrou **partagé**, pour toute sa durée :
  plusieurs sessions peuvent tourner ensemble.
- La remise à zéro prend le verrou **exclusif**, sans attendre : si une session
  tourne, elle refuse aussitôt plutôt que d'attendre une suite de huit minutes.

# Pourquoi `fcntl` et pas `flock(1)`

La commande `flock` n'existe pas sur macOS. `fcntl.flock` est dans la
bibliothèque standard, sur macOS comme sur Linux (CI) : un seul mécanisme des
deux côtés, et aucun paquet à installer.

# Pourquoi `/tmp` et pas `$TMPDIR`

`$TMPDIR` dépend de la session : sur macOS il pointe dans `/var/folders/…`, et
un outil peut le déplacer pour ses propres processus. Deux processus aux
`$TMPDIR` différents prendraient deux verrous différents — chacun croirait la
base à lui. `/tmp` est le même pour tous. Le nom du fichier est dérivé de la
base visée (hôte, port, nom — jamais les identifiants) : deux bases distinctes
ne se bloquent pas l'une l'autre.

Usage en ligne de commande, par le script de remise à zéro :

    python3 tests/e2e/e2e_lock.py exclusive -- <commande> [args…]
"""

import fcntl
import os
import re
import subprocess
import sys
import time
from pathlib import Path
from urllib.parse import urlsplit

from e2e_env import database_url

# Code de sortie quand le verrou exclusif est pris : EX_TEMPFAIL, « réessayez ».
BUSY_EXIT_CODE = 75

# Variable posée pour la commande lancée sous le verrou exclusif : le script de
# remise à zéro s'en sert pour savoir qu'il est déjà protégé.
HELD_ENV_VAR = "KREEK_E2E_LOCK_HELD"


def lock_path() -> Path:
    """Le fichier de verrou de la base e2e — surchargeable par `KREEK_E2E_LOCK`."""
    override = os.environ.get("KREEK_E2E_LOCK")
    if override:
        return Path(override)
    return Path("/tmp") / f"kreek-e2e-{_database_identity(database_url())}.lock"


def _database_identity(url: str) -> str:
    """`hôte-port-base`, sans les identifiants que l'URL porte."""
    parts = urlsplit(url)
    ident = f"{parts.hostname or 'local'}-{parts.port or 5432}-{parts.path.lstrip('/')}"
    return re.sub(r"[^A-Za-z0-9_.-]", "_", ident)


def _open_lock_file() -> int:
    return os.open(lock_path(), os.O_RDWR | os.O_CREAT, 0o666)


def try_lock(mode: int):
    """Le descripteur verrouillé, ou `None` si le verrou est pris ailleurs.

    Le verrou vit autant que le descripteur : le fermer, ou finir le processus,
    le relâche — y compris sur un plantage.
    """
    fd = _open_lock_file()
    try:
        fcntl.flock(fd, mode | fcntl.LOCK_NB)
    except BlockingIOError:
        os.close(fd)
        return None
    return fd


def acquire_shared(timeout_s: float):
    """Verrou partagé d'une session pytest — attend la fin d'une remise à zéro.

    Une remise à zéro dure quelques secondes ; on l'attend plutôt que d'échouer.
    Rend `None` si elle ne finit pas dans le délai.
    """
    limite = time.time() + timeout_s
    while True:
        fd = try_lock(fcntl.LOCK_SH)
        if fd is not None or time.time() >= limite:
            return fd
        time.sleep(0.5)


def run_exclusive(cmd: list) -> int:
    """Exécute `cmd` sous le verrou exclusif, ou refuse sans attendre."""
    fd = try_lock(fcntl.LOCK_EX)
    if fd is None:
        print(
            f"\n  ✗ Refus : une autre suite e2e utilise la base (verrou {lock_path()}).\n"
            "    Attendez qu'elle finisse — la base ne peut pas être vidée sous elle.\n",
            file=sys.stderr,
        )
        return BUSY_EXIT_CODE
    env = {**os.environ, HELD_ENV_VAR: "exclusive"}
    return subprocess.run(cmd, env=env).returncode


def _main(argv: list) -> int:
    if len(argv) < 3 or argv[0] != "exclusive" or argv[1] != "--":
        print("usage : e2e_lock.py exclusive -- <commande> [args…]", file=sys.stderr)
        return 2
    return run_exclusive(argv[2:])


if __name__ == "__main__":
    sys.exit(_main(sys.argv[1:]))
