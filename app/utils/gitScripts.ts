/**
 * Shell scripts for git diff/snapshot operations.
 * Executed via PTY oneshot to capture file contents as base64-encoded before/after pairs.
 */

/** Script to snapshot a single commit's diff (before/after for each file). */
export const COMMIT_SNAPSHOT_SCRIPT = [
  'stty -opost -echo 2>/dev/null',
  'export GIT_PAGER=cat',
  'export GIT_TERMINAL_PROMPT=0',
  'h=$1',
  'printf "##TITLE\\t%s\\n" "$(git --no-pager log --format="%h %s" -1 "$h" 2>/dev/null)"',
  'git diff-tree --no-commit-id -r --name-status --find-renames --find-copies --first-parent --root "$h" 2>/dev/null | while IFS="$(printf "\\t")" read -r st p1 p2; do',
  '  code=${st%"${st#?}"}',
  '  old=$p1',
  '  new=$p1',
  '  if [ "$code" = "R" ] || [ "$code" = "C" ]; then',
  '    old=$p1',
  '    new=$p2',
  '  fi',
  '  printf "##FILE\\t%s\\t%s\\n" "$st" "$new"',
  '  printf "##BEFORE\\n"',
  '  if [ "$code" != "A" ]; then',
  '    git --no-pager show "$h^:$old" 2>/dev/null | base64 -w 76',
  '  fi',
  '  printf "##AFTER\\n"',
  '  if [ "$code" != "D" ]; then',
  '    git --no-pager show "$h:$new" 2>/dev/null | base64 -w 76',
  '  fi',
  'done',
].join('\n');

/** Script to snapshot a single file (before/after for staged or unstaged). */
export const FILE_SNAPSHOT_SCRIPT = [
  'stty -opost -echo 2>/dev/null',
  'export GIT_PAGER=cat',
  'export GIT_TERMINAL_PROMPT=0',
  'mode=$1',
  'path=$2',
  'printf "##BEFORE\\n"',
  'if [ "$mode" = "staged" ]; then',
  '  git --no-pager show "HEAD:$path" 2>/dev/null | base64 -w 76',
  'else',
  '  git --no-pager show ":$path" 2>/dev/null | base64 -w 76',
  'fi',
  'printf "##AFTER\\n"',
  'if [ "$mode" = "staged" ]; then',
  '  git --no-pager show ":$path" 2>/dev/null | base64 -w 76',
  'else',
  '  if [ -f "$path" ]; then',
  '    base64 -w 76 < "$path"',
  '  fi',
  'fi',
].join('\n');

export type WorktreeSnapshotMode = 'staged' | 'changes' | 'all';

/** Build a script to snapshot the whole worktree diff (staged, unstaged, or both). */
export function buildWorktreeSnapshotScript(mode: WorktreeSnapshotMode): string {
  const title =
    mode === 'staged'
      ? 'Staged changes'
      : mode === 'changes'
        ? 'Unstaged changes'
        : 'Working tree (staged + changes)';

  let filterLines: string[];
  if (mode === 'staged') {
    filterLines = ['  [ "$x" = " " ] && continue', '  [ "$x" = "?" ] && continue'];
  } else if (mode === 'changes') {
    filterLines = ['  [ "$y" = " " ] && continue', '  [ "$y" = "?" ] && continue'];
  } else {
    filterLines = ['  [ "$x" = "?" ] && [ "$y" = "?" ] && continue'];
  }

  let beforeLines: string[];
  let afterLines: string[];
  if (mode === 'staged') {
    beforeLines = [
      '  printf "##BEFORE\\n"',
      '  if [ "$code" != "A" ]; then',
      '    git --no-pager show "HEAD:$old" 2>/dev/null | base64 -w 76',
      '  fi',
    ];
    afterLines = [
      '  printf "##AFTER\\n"',
      '  if [ "$code" != "D" ]; then',
      '    git --no-pager show ":$new" 2>/dev/null | base64 -w 76',
      '  fi',
    ];
  } else if (mode === 'changes') {
    beforeLines = [
      '  printf "##BEFORE\\n"',
      '  if [ "$code" != "A" ]; then',
      '    git --no-pager show ":$old" 2>/dev/null | base64 -w 76',
      '  fi',
    ];
    afterLines = [
      '  printf "##AFTER\\n"',
      '  if [ "$code" != "D" ] && [ -f "$new" ]; then',
      '    base64 -w 76 < "$new"',
      '  fi',
    ];
  } else {
    beforeLines = [
      '  printf "##BEFORE\\n"',
      '  if [ "$code" != "A" ]; then',
      '    git --no-pager show "HEAD:$old" 2>/dev/null | base64 -w 76',
      '  fi',
    ];
    afterLines = [
      '  printf "##AFTER\\n"',
      '  if [ "$code" != "D" ] && [ -f "$new" ]; then',
      '    base64 -w 76 < "$new"',
      '  fi',
    ];
  }

  return [
    'stty -opost -echo 2>/dev/null',
    'export GIT_PAGER=cat',
    'export GIT_TERMINAL_PROMPT=0',
    `printf "##TITLE\\t${title}\\n"`,
    'git --no-pager status --porcelain=v1 2>/dev/null | while IFS= read -r line; do',
    '  [ -z "$line" ] && continue',
    '  x=${line%"${line#?}"}',
    '  rest=${line#?}',
    '  y=${rest%"${rest#?}"}',
    ...filterLines,
    '  path=${line#???}',
    '  old=$path',
    '  new=$path',
    '  code=M',
    '  if [ "$x" = "D" ] || [ "$y" = "D" ]; then',
    '    code=D',
    '  elif [ "$x" = "A" ]; then',
    '    code=A',
    '  elif [ "$x" = "R" ] || [ "$y" = "R" ]; then',
    '    code=R',
    '    old=${path%% -> *}',
    '    new=${path#* -> }',
    '  elif [ "$x" = "C" ] || [ "$y" = "C" ]; then',
    '    code=C',
    '    old=${path%% -> *}',
    '    new=${path#* -> }',
    '  fi',
    '  printf "##FILE\\t%s\\t%s\\n" "$code" "$new"',
    ...beforeLines,
    ...afterLines,
    'done',
  ].join('\n');
}
