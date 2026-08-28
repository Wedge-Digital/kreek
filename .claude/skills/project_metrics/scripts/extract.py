#!/usr/bin/env python3
"""Extrait de git la série hebdomadaire de vélocité d'un dépôt.

Sortie : un JSON sur stdout (ou dans --out) plus un rapport lisible sur stderr.
Toutes les métriques sont documentées dans SKILL.md ; les seuils qui font la
mesure (session, semaine creuse, forfait d'amorce) sont les constantes ci-dessous.
"""
import argparse, json, math, re, subprocess, sys
import datetime as dt
from collections import defaultdict

SESSION_GAP_MIN = 90    # au-delà, deux commits appartiennent à deux sessions
SESSION_LEAD_MIN = 15   # forfait attribué au premier commit d'une session
WEEK_MIN_COMMITS = 10   # en deçà, la semaine mesure une absence, pas un rythme

# --- ce qui est spécifique au dépôt -----------------------------------------
DOC_PREFIXES = ("kanban/", "docs/")          # commits qui ne portent pas de code
CATEGORIES = [                               # premier motif qui accroche gagne
    ("docs",  lambda f: f.startswith(DOC_PREFIXES) or f.endswith(".md")),
    ("e2e",   lambda f: f.startswith("tests/")),
    ("outil", lambda f: f.startswith(("scripts/", ".github", ".githooks", "config/", ".env"))
                        or f == "Makefile" or f.endswith((".toml", ".yml", ".yaml"))
                        or f.startswith("migrations/") or f.endswith(".sql")),
    ("rs",    lambda f: f.endswith(".rs")),   # scindé prod / tests unitaires
    ("front", lambda f: f.endswith((".html", ".css", ".js"))),
]
CODE_GLOB = "src/*.rs"                        # ce dont on mesure la masse
CAT_LABELS = {"prod": "code de production", "test_u": "tests unitaires",
              "front": "templates & CSS", "e2e": "tests e2e",
              "outil": "outillage & CI", "docs": "kanban & règles", "autre": "divers"}
STACK_ORDER = ["prod", "front", "test_u", "e2e", "outil", "docs", "autre"]


REPO = "."   # posé par main() ; toutes les commandes git passent par -C


def git(*args, binary=False, stdin=None):
    r = subprocess.run(["git", "-C", REPO, *args], input=stdin, capture_output=True)
    if r.returncode:
        sys.exit(f"git {' '.join(args)} : {r.stderr.decode()[:400]}")
    return r.stdout if binary else r.stdout.decode("utf-8", "replace")


def is_code(path):
    return not (path.startswith(DOC_PREFIXES) or path.endswith(".md"))


def categorise(path):
    for name, match in CATEGORIES:
        if match(path):
            return name
    return "autre"


def read_commits(since):
    """Toutes les révisions depuis `since`, avec leur numstat par fichier."""
    raw = git("log", "--reverse", f"--since={since}", "--pretty=format:X|%H|%aI|%s", "--numstat")
    commits, cur = [], None
    for line in raw.split("\n"):
        if line.startswith("X|"):
            _, sha, when, subject = line.split("|", 3)
            cur = {"sha": sha, "dt": dt.datetime.fromisoformat(when),
                   "subject": subject, "add": {}, "churn": {}}
            commits.append(cur)
        elif line.strip() and cur is not None:
            parts = line.split("\t")
            if len(parts) == 3 and parts[0].isdigit():
                added, removed, path = int(parts[0]), int(parts[1]), parts[2]
                cur["add"][path] = cur["add"].get(path, 0) + added
                cur["churn"][path] = cur["churn"].get(path, 0) + added + removed
    commits.sort(key=lambda c: c["dt"])
    return commits


def attribute_time(commits):
    """Pose sur chaque commit le temps qu'il a coûté, et renvoie les sessions."""
    if not commits:
        return []
    sessions, current = [], [commits[0]]
    for previous, nxt in zip(commits, commits[1:]):
        if (nxt["dt"] - previous["dt"]).total_seconds() / 60 <= SESSION_GAP_MIN:
            current.append(nxt)
        else:
            sessions.append(current)
            current = [nxt]
    sessions.append(current)
    for session in sessions:
        session[0]["t"] = float(SESSION_LEAD_MIN)
        for previous, nxt in zip(session, session[1:]):
            nxt["t"] = (nxt["dt"] - previous["dt"]).total_seconds() / 60
    return sessions


def week_of(moment):
    return moment.isocalendar()[:2]          # (année ISO, semaine ISO)


def week_end_shas(commits):
    """Dernier commit de chaque semaine ISO — l'état du dépôt en fin de semaine."""
    last = {}
    for c in commits:
        last[week_of(c["dt"])] = c["sha"]
    return last


def count_rust_split(sha):
    """Lignes de src/**/*.rs hors et dans les modules #[cfg(test)]."""
    tree = git("ls-tree", "-r", sha, "--", CODE_GLOB.split("/")[0]).strip().split("\n")
    blobs = [l.split("\t")[0].split()[2] for l in tree
             if l.strip() and l.split("\t", 1)[1].endswith(".rs")]
    if not blobs:
        return 0, 0
    out = git("cat-file", "--batch", binary=True,
              stdin=("\n".join(blobs) + "\n").encode())
    prod = test = 0
    pos = 0
    for _ in blobs:
        nl = out.index(b"\n", pos)
        size = int(out[pos:nl].split()[2])          # en octets, pas en caractères
        body = out[nl + 1:nl + 1 + size].decode("utf-8", "replace")
        pos = nl + 1 + size + 1
        p, t = split_test_modules(body)
        prod += p
        test += t
    return prod, test


def split_test_modules(source):
    """Compte les lignes dans / hors les blocs #[cfg(test)], par accolades."""
    lines = source.split("\n")
    prod = test = depth = 0
    inside = False
    i = 0
    while i < len(lines):
        line = lines[i]
        if not inside and "#[cfg(test)]" in line:
            j = i
            while j < len(lines) and "{" not in lines[j]:
                j += 1
            if j < len(lines):
                inside, depth = True, 0
                for k in range(i, j + 1):
                    depth += lines[k].count("{") - lines[k].count("}")
                    test += 1
                i = j + 1
                continue
        if inside:
            depth += line.count("{") - line.count("}")
            test += 1
            inside = depth > 0
        else:
            prod += 1
        i += 1
    return prod, test


def weekly_rhythm(commits):
    """Commits, sessions, heures actives et taille des commits, par semaine."""
    weeks = defaultdict(lambda: {"commits": 0, "hours": 0.0, "days": set(),
                                 "sizes": [], "add": 0, "net": 0, "sessions": 0})
    for c in commits:
        if not any(is_code(f) for f in c["churn"]):
            continue
        w = weeks[week_of(c["dt"])]
        added = sum(v for f, v in c["add"].items() if is_code(f))
        churn = sum(v for f, v in c["churn"].items() if is_code(f))
        w["commits"] += 1
        w["hours"] += c["t"] / 60
        w["days"].add(c["dt"].date())
        w["sizes"].append(added)
        w["add"] += added
        w["net"] += 2 * added - churn          # ajouts − suppressions
        w["sessions"] += 1 if c["t"] == SESSION_LEAD_MIN else 0
    return weeks


def weekly_time_split(commits, test_ratio):
    """Heures actives par catégorie, réparties au prorata des lignes remuées."""
    weeks = defaultdict(lambda: defaultdict(float))
    for c in commits:
        w = week_of(c["dt"])
        total = sum(c["churn"].values())
        if total == 0:
            weeks[w]["autre"] += c["t"] / 60
            continue
        per_cat = defaultdict(float)
        for path, churn in c["churn"].items():
            per_cat[categorise(path)] += churn
        for cat, churn in per_cat.items():
            hours = c["t"] / 60 * churn / total
            if cat == "rs":
                ratio = test_ratio.get(w, 0.0)
                weeks[w]["test_u"] += hours * ratio
                weeks[w]["prod"] += hours * (1 - ratio)
            else:
                weeks[w][cat] += hours
    return weeks


def dominant_time_split(commits, test_ratio, keep):
    """Contrôle : tout le temps d'un commit va à sa catégorie dominante."""
    totals = defaultdict(float)
    for c in commits:
        w = week_of(c["dt"])
        if w not in keep or not c["churn"]:
            continue
        per_cat = defaultdict(float)
        for path, churn in c["churn"].items():
            per_cat[categorise(path)] += churn
        dom = max(per_cat, key=per_cat.get)
        if dom == "rs":
            ratio = test_ratio.get(w, 0.0)
            totals["test_u"] += c["t"] * ratio
            totals["prod"] += c["t"] * (1 - ratio)
        else:
            totals[dom] += c["t"]
    grand = sum(totals.values()) or 1
    return {k: 100 * v / grand for k, v in totals.items()}


def test_ratio_by_week(sizes, weeks_sorted):
    """Part des tests unitaires dans le Rust ajouté chaque semaine."""
    ratios = {}
    for i, w in enumerate(weeks_sorted):
        prod, test = sizes[w]["prod_rs"], sizes[w]["test_rs"]
        if i:
            prev = sizes[weeks_sorted[i - 1]]
            prod, test = prod - prev["prod_rs"], test - prev["test_rs"]
        prod, test = max(prod, 0), max(test, 0)
        if prod + test:
            ratios[w] = test / (prod + test)
        else:
            state = sizes[w]
            ratios[w] = state["test_rs"] / max(1, state["prod_rs"] + state["test_rs"])
    return ratios


def median(values):
    v = sorted(values)
    n = len(v)
    return 0 if not n else (v[n // 2] if n % 2 else (v[n // 2 - 1] + v[n // 2]) / 2)


def log_fit(rows):
    """min/commit ≈ a + b·log2(kLOC) sur les semaines pleines. Renvoie a, b, R², r."""
    pts = [(math.log2(r["kloc"]), r["min_per_commit"]) for r in rows if r["kloc"] > 0]
    n = len(pts)
    if n < 3:
        return None
    mx = sum(x for x, _ in pts) / n
    my = sum(y for _, y in pts) / n
    sxy = sum((x - mx) * (y - my) for x, y in pts)
    sxx = sum((x - mx) ** 2 for x, _ in pts)
    syy = sum((y - my) ** 2 for _, y in pts)
    b = sxy / sxx
    a = my - b * mx
    sse = sum((y - (a + b * x)) ** 2 for x, y in pts)
    return {"a": a, "b": b, "r2": 1 - sse / syy, "r": sxy / math.sqrt(sxx * syy), "n": n}


def build_rows(commits, sizes, rhythm, split):
    rows = []
    for w in sorted(rhythm):
        r, s, t = rhythm[w], sizes.get(w, {}), split.get(w, {})
        hours = r["hours"] or 0.001
        rows.append({
            "week": f"{w[0]}-S{w[1]:02d}",
            "start": dt.date.fromisocalendar(w[0], w[1], 1).isoformat(),
            "kloc": round((s.get("prod_rs", 0) + s.get("test_rs", 0)) / 1000, 1),
            "prod_kloc": round(s.get("prod_rs", 0) / 1000, 1),
            "test_kloc": round(s.get("test_rs", 0) / 1000, 1),
            "commits": r["commits"], "sessions": r["sessions"], "days": len(r["days"]),
            "hours": round(hours, 1),
            "min_per_commit": round(hours * 60 / r["commits"], 1),
            "commits_per_hour": round(r["commits"] / hours, 2),
            "lines_per_commit": round(median(r["sizes"])),
            "added": r["add"], "net": r["net"],
            "min_per_100_lines": round(hours * 6000 / max(1, r["add"]), 2),
            "time": {k: round(t.get(k, 0.0), 2) for k in STACK_ORDER},
            "sparse": r["commits"] < WEEK_MIN_COMMITS,
        })
    return rows


def plateaus(rows, cut):
    full = [r for r in rows if not r["sparse"]]
    if len(full) < 4:
        return [], []
    cut = cut or len(full) // 2
    return full[:cut], full[cut:]


def compare(a, b):
    """Compare deux plateaux. Les ratios sont agrégés, jamais moyennés :
    la moyenne de rapports hebdomadaires laisse une semaine creuse peser
    autant qu'une semaine pleine."""
    mean = lambda rows, k: sum(r[k] for r in rows) / len(rows)
    total = lambda rows, k: sum(r[k] for r in rows)
    derived = {
        "min_per_commit":    lambda r: total(r, "hours") * 60 / total(r, "commits"),
        "commits_per_hour":  lambda r: total(r, "commits") / total(r, "hours"),
        "min_per_100_lines": lambda r: total(r, "hours") * 6000 / total(r, "added"),
    }
    out = {}
    for key in ("kloc", "hours", "lines_per_commit", "net"):
        va, vb = mean(a, key), mean(b, key)
        out[key] = {"a": round(va, 2), "b": round(vb, 2),
                    "ratio": round(vb / va, 2) if va else None}
    for key, fn in derived.items():
        va, vb = fn(a), fn(b)
        out[key] = {"a": round(va, 2), "b": round(vb, 2),
                    "ratio": round(vb / va, 2) if va else None}
    shares = []
    for rows in (a, b):
        grand = sum(sum(r["time"].values()) for r in rows) or 1
        shares.append({k: round(100 * sum(r["time"][k] for r in rows) / grand, 1)
                       for k in STACK_ORDER})
    out["time_share"] = {"a": shares[0], "b": shares[1]}
    return out


def report(rows, comp, fit, control, out):
    p = lambda *a: print(*a, file=sys.stderr)
    p(f"\n{'semaine':10}{'kLOC':>7}{'com':>5}{'sess':>5}{'h':>7}{'min/com':>9}"
      f"{'com/h':>7}{'lig/com':>9}{'min/100l':>10}")
    for r in rows:
        mark = "·" if r["sparse"] else " "
        p(f"{r['start']:10}{r['kloc']:7.1f}{r['commits']:5}{r['sessions']:5}"
          f"{r['hours']:7.1f}{r['min_per_commit']:9.1f}{r['commits_per_hour']:7.2f}"
          f"{r['lines_per_commit']:9}{r['min_per_100_lines']:9.2f}{mark}")
    p("\n· semaine creuse (< %d commits), exclue des moyennes" % WEEK_MIN_COMMITS)
    if not comp:
        return
    p(f"\n{'':22}{'plateau A':>12}{'plateau B':>12}{'':>8}")
    for k, v in comp.items():
        if k == "time_share":
            continue
        p(f"{k:22}{v['a']:12.2f}{v['b']:12.2f}   ×{v['ratio']}")
    p(f"\n--- part du temps actif ---{'':10}{'A':>8}{'B':>9}{'contrôle B':>12}")
    for k in STACK_ORDER:
        p(f"{CAT_LABELS[k]:34}{comp['time_share']['a'][k]:7.1f}%"
          f"{comp['time_share']['b'][k]:8.1f}%{control.get(k, 0):11.1f}%")
    if fit:
        p(f"\nmin/commit ≈ {fit['a']:.1f} + {fit['b']:.2f}·log2(kLOC)"
          f"   r={fit['r']:.2f}  R²={fit['r2']:.2f}  n={fit['n']}")
        p(f"→ +{fit['b']:.1f} min par commit à chaque doublement de la base de code")
    p(f"\nJSON écrit dans {out}")


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--since", default="12 months ago", help="borne basse de l'historique")
    ap.add_argument("--cut", type=int, help="index de coupure entre les deux plateaux")
    ap.add_argument("--out", default="velocity.json")
    ap.add_argument("--repo", default=".", help="racine du dépôt à mesurer")
    args = ap.parse_args()

    global REPO
    REPO = args.repo
    if subprocess.run(["git", "-C", REPO, "rev-parse", "--git-dir"],
                      capture_output=True).returncode:
        sys.exit(f"{REPO} n'est pas un dépôt git — passer --repo <racine>")

    commits = read_commits(args.since)
    if not commits:
        sys.exit("aucun commit sur la période")
    attribute_time(commits)

    sizes = {}
    for w, sha in week_end_shas(commits).items():
        prod, test = count_rust_split(sha)
        sizes[w] = {"prod_rs": prod, "test_rs": test}
    weeks_sorted = sorted(sizes)
    ratios = test_ratio_by_week(sizes, weeks_sorted)

    rhythm = weekly_rhythm(commits)
    split = weekly_time_split(commits, ratios)
    rows = build_rows(commits, sizes, rhythm, split)

    a, b = plateaus(rows, args.cut)
    comp = compare(a, b) if a and b else {}
    fit = log_fit([r for r in rows if not r["sparse"]])
    keep_b = {tuple(int(x) for x in r["week"].replace("S", "").split("-")) for r in b}
    control = dominant_time_split(commits, ratios, keep_b) if b else {}

    payload = {"generated": dt.datetime.now().isoformat(timespec="seconds"),
               "since": args.since, "weeks": rows, "plateaus": comp,
               "fit": fit, "control_dominant": control,
               "labels": CAT_LABELS, "stack_order": STACK_ORDER}
    with open(args.out, "w") as fh:
        json.dump(payload, fh, indent=1, ensure_ascii=False)
    report(rows, comp, fit, control, args.out)


if __name__ == "__main__":
    main()
