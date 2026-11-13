/** Lesson content. Puzzle lessons pull verified puzzles from the Lichess database; the basics lesson and endgames are hand-made and checked by tests. */

export type Level = "Beginner" | "Improver" | "Intermediate";

export type PuzzleLesson = {
  kind: "puzzles";
  id: string;
  title: string;
  icon: string;
  level: Level;
  blurb: string;
  /** Lichess theme the puzzles must have, and an optional second tag (for example oneMove). */
  theme: string;
  alsoTag?: string;
  minRating: number;
  maxRating: number;
  count: number;
  intro: string[];
  ideas: string[];
  /** Shown after each solved puzzle. */
  solved: string;
};

export type Step = {
  id: string;
  fen: string;
  prompt: string;
  hint: string;
  /** Accepted moves (UCI), or "any" when every legal move is fine (escaping check). */
  accept: string[] | "any";
  explain: string;
};

export type StepsLesson = {
  kind: "steps";
  id: string;
  title: string;
  icon: string;
  level: Level;
  blurb: string;
  intro: string[];
  ideas: string[];
  steps: Step[];
};

export type Lesson = PuzzleLesson | StepsLesson;

export const BASICS: StepsLesson = {
  kind: "steps",
  id: "basics",
  title: "How the pieces move",
  icon: "♞",
  level: "Beginner",
  blurb: "Rook, bishop, knight, queen, king, pawns and the special moves, one tiny exercise each.",
  intro: [
    "Every piece moves in its own way. In each exercise you play White and make the move the question asks for.",
    "Pieces capture by moving onto an enemy piece. A king can never move into check, and when your king is attacked you must fix it straight away.",
  ],
  ideas: ["Rooks go straight, bishops go diagonally, queens do both.", "Knights jump in an L shape and can hop over pieces.", "Pawns move forward but capture diagonally."],
  steps: [
    { id: "rook", fen: "7k/3p4/8/8/3R4/8/8/K7 w - - 0 1", prompt: "Capture the pawn with your rook.", hint: "Rooks move in straight lines along files and ranks.", accept: ["d4d7"], explain: "Rooks slide any distance along a file or rank." },
    { id: "bishop", fen: "7k/8/8/3p4/8/8/B7/K7 w - - 0 1", prompt: "Capture the pawn with your bishop.", hint: "Bishops move on diagonals.", accept: ["a2d5"], explain: "Bishops slide along diagonals and always stay on the same colour of square." },
    { id: "knight", fen: "7k/8/8/8/3p4/8/4N3/K7 w - - 0 1", prompt: "Capture the pawn with your knight.", hint: "Two squares one way, then one square sideways.", accept: ["e2d4"], explain: "The knight moves in an L: two squares and one to the side. It is the only piece that can jump over others." },
    { id: "queen", fen: "7k/8/8/7r/8/8/8/K2Q4 w - - 0 1", prompt: "Capture the rook with your queen.", hint: "The queen moves like a rook and a bishop together.", accept: ["d1h5"], explain: "The queen is the strongest piece: any distance in a straight line or diagonal." },
    { id: "check", fen: "4r2k/8/8/8/8/8/8/4K3 w - - 0 1", prompt: "Your king is in check from the rook! Move it to safety.", hint: "Step off the e-file, away from the rook.", accept: "any", explain: "When your king is in check you must get out of it: move the king, block, or capture the attacker." },
    { id: "promote", fen: "8/4P1k1/8/8/8/8/8/K7 w - - 0 1", prompt: "Push your pawn to the last rank and promote it to a queen.", hint: "Move the pawn from e7 to e8 and choose Queen.", accept: ["e7e8q"], explain: "A pawn that reaches the far side turns into a queen, rook, bishop or knight. Almost always: queen." },
    { id: "castle", fen: "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1", prompt: "Castle kingside (short): move your king two squares towards the h-file rook.", hint: "Drag the king from e1 to g1.", accept: ["e1g1"], explain: "Castling tucks your king away and brings a rook to the middle. It is allowed only if neither piece has moved and no square in between is attacked." },
    { id: "enpassant", fen: "7k/8/8/3pP3/8/8/8/K7 w - d6 0 1", prompt: "Black just played d7-d5. Capture it \"en passant\" with your e5 pawn.", hint: "Move the pawn diagonally to d6, behind the black pawn.", accept: ["e5d6"], explain: "If an enemy pawn steps two squares past your pawn, you may capture it as if it had moved one square. You have to do it right away." },
  ],
};

export const PUZZLE_LESSONS: PuzzleLesson[] = [
  {
    kind: "puzzles", id: "mate-in-1", title: "Checkmate in one", icon: "♚", level: "Beginner", theme: "mateIn1", alsoTag: "oneMove", minRating: 400, maxRating: 900, count: 8,
    blurb: "Find the move that ends the game on the spot.",
    intro: ["Checkmate means the king is attacked and has no escape. The game is over at once.", "Before every move ask: can I give check, and can the king get away? Look at every check you have."],
    ideas: ["Start by listing all your checks.", "Look for a king boxed in by its own pieces.", "A queen next to the king is often protected by another piece."],
    solved: "Checkmate: the king was attacked and could not escape.",
  },
  {
    kind: "puzzles", id: "back-rank", title: "Back-rank mates", icon: "🏰", level: "Beginner", theme: "backRankMate", alsoTag: "mateIn1", minRating: 400, maxRating: 1000, count: 8,
    blurb: "A king trapped behind its own pawns is an easy target.",
    intro: ["When a king sits on its back rank with its own pawns in front of it, a rook or queen check along that rank can be mate.", "Protect your own back rank by giving your king an escape square, and look for the same weakness in your opponent's camp."],
    ideas: ["Look for kings with no luft (escape square).", "A rook or queen on the back rank is the usual finisher.", "Remove or distract the defender, then deliver mate."],
    solved: "Back-rank mate: the pawns in front of the king took away its escape squares.",
  },
  {
    kind: "puzzles", id: "hanging", title: "Free pieces", icon: "🎁", level: "Beginner", theme: "hangingPiece", minRating: 400, maxRating: 800, count: 8,
    blurb: "Spot the piece nobody is protecting and take it.",
    intro: ["A hanging piece is one that can be captured for free. Winning material is the simplest way to win games.", "Before you move, check what your opponent just left unprotected, and check that your own pieces are safe."],
    ideas: ["Ask after every move: what is attacked, and is it defended?", "Capture the most valuable piece you can safely win.", "Always check your own pieces before moving."],
    solved: "You won material by finding an unprotected piece.",
  },
  {
    kind: "puzzles", id: "forks", title: "Forks", icon: "🍴", level: "Beginner", theme: "fork", minRating: 500, maxRating: 1000, count: 8,
    blurb: "One piece attacks two things at once.",
    intro: ["A fork is a move where one piece attacks two (or more) enemy pieces at the same time. Your opponent can only save one.", "Knights are famous for forks because they attack in an unusual pattern, but every piece can fork, even pawns."],
    ideas: ["Look for two targets that are undefended or valuable.", "A knight fork on king and queen is a classic.", "Checks are the best forking moves: the opponent must answer the check."],
    solved: "Fork: one piece attacked two targets, so you win one of them.",
  },
  {
    kind: "puzzles", id: "pins", title: "Pins", icon: "📌", level: "Improver", theme: "pin", minRating: 600, maxRating: 1100, count: 8,
    blurb: "A piece that cannot move without exposing something more valuable.",
    intro: ["A pinned piece is stuck in front of a more valuable piece (often the king). Moving it would lose that piece.", "Bishops, rooks and queens create pins because they attack along lines. Attack the pinned piece again and win it."],
    ideas: ["A piece pinned to the king cannot legally move.", "Pile up attackers on the pinned piece.", "Pins can also win material when the piece behind is worth more."],
    solved: "Pin: the pinned piece could not move without losing something bigger.",
  },
  {
    kind: "puzzles", id: "skewers", title: "Skewers", icon: "🗡️", level: "Improver", theme: "skewer", minRating: 600, maxRating: 1200, count: 8,
    blurb: "The reverse of a pin: the big piece must move, exposing the one behind.",
    intro: ["A skewer attacks a valuable piece that has to move away, and then you capture the piece behind it.", "It works along files, ranks and diagonals, with a bishop, rook or queen."],
    ideas: ["Line up your rook or bishop with a king or queen.", "After the valuable piece moves, take what was behind.", "Check first: a skewered king must move."],
    solved: "Skewer: the valuable piece moved and you captured the one behind it.",
  },
  {
    kind: "puzzles", id: "discovered", title: "Discovered attacks", icon: "🎭", level: "Improver", theme: "discoveredAttack", minRating: 700, maxRating: 1300, count: 8,
    blurb: "Move one piece to unmask another.",
    intro: ["In a discovered attack you move a piece out of the way and reveal an attack from the piece behind it. Now you threaten two things at once.", "The best version is a discovered check: the opponent must answer the check while your moved piece grabs something."],
    ideas: ["Look for your bishop, rook or queen hidden behind your own piece.", "The piece you move can attack something too.", "A discovered check wins a tempo."],
    solved: "Discovered attack: moving one piece unmasked a second attacker.",
  },
  {
    kind: "puzzles", id: "mate-in-2", title: "Checkmate in two", icon: "♛", level: "Improver", theme: "mateIn2", minRating: 600, maxRating: 1200, count: 8,
    blurb: "A forcing first move, then mate.",
    intro: ["In a mate in two you make a first move that leaves your opponent no good answer, then deliver checkmate.", "Think in forcing moves: checks, captures and threats. If you take away every escape, the mate follows."],
    ideas: ["Start with checks, then captures, then threats.", "Imagine the position after the opponent's reply.", "Quiet moves that threaten mate can be as strong as checks."],
    solved: "Mate in two: your first move left no defence.",
  },
  {
    kind: "puzzles", id: "trapped", title: "Trapped pieces", icon: "🪤", level: "Improver", theme: "trappedPiece", minRating: 700, maxRating: 1300, count: 8,
    blurb: "A piece with nowhere safe to go.",
    intro: ["A trapped piece is attacked and has no safe square to run to. Win it before it escapes.", "Bishops and knights near the edge, and queens that wander too far, are the most common victims."],
    ideas: ["Count a piece's safe squares.", "Attack it with a cheaper piece.", "Take away the escape squares before you attack."],
    solved: "Trapped piece: every escape square was covered.",
  },
  {
    kind: "puzzles", id: "promotion", title: "Pawn promotion", icon: "👑", level: "Improver", theme: "promotion", minRating: 500, maxRating: 1100, count: 8,
    blurb: "Get a pawn all the way to the end.",
    intro: ["A pawn that reaches the last rank becomes a queen (usually). Passed pawns, with no enemy pawns in front, are dangerous.", "Clear the path, block the defenders, and push."],
    ideas: ["Count how many moves the pawn needs.", "Remove the pieces guarding the promotion square.", "Sometimes promoting to a knight gives check."],
    solved: "Promotion: the pawn reached the end and became a queen.",
  },
  {
    kind: "puzzles", id: "deflection", title: "Deflection and sacrifice", icon: "💥", level: "Intermediate", theme: "deflection", minRating: 900, maxRating: 1500, count: 6,
    blurb: "Drag a defender away, or give something up to win more.",
    intro: ["Deflection forces a defending piece away from its job. A sacrifice gives up material to open lines or win something bigger.", "Ask: who is defending the key square, and can I force them to leave?"],
    ideas: ["Find the piece holding everything together.", "Check what happens after the capture.", "Count the material at the end, not the start."],
    solved: "Deflection: the defender was forced away and everything fell apart.",
  },
  {
    kind: "puzzles", id: "quiet", title: "Quiet moves", icon: "🤫", level: "Intermediate", theme: "quietMove", minRating: 900, maxRating: 1500, count: 6,
    blurb: "Not a check, not a capture, and crushing anyway.",
    intro: ["A quiet move creates an unstoppable threat without capturing or giving check. They are hard to see because we look for forcing moves first.", "When the forcing moves do not work, ask: what is my opponent's only defence, and can I take it away calmly?"],
    ideas: ["After checks and captures, look at threats.", "Improve your worst piece.", "Look for moves that leave your opponent in zugzwang or with no good moves."],
    solved: "A quiet move: no check, no capture, and still decisive.",
  },
];

export const LESSONS: Lesson[] = [BASICS, ...PUZZLE_LESSONS];
export const getLesson = (id: string): Lesson | undefined => LESSONS.find((l) => l.id === id);

// ---------------------------------------------------------------------------------------------------- endgames

export type Endgame = {
  id: string;
  title: string;
  level: Level;
  fen: string;
  /** The side you play. */
  player: "white" | "black";
  goal: "win" | "draw";
  blurb: string;
  tip: string;
};

/** Each position was checked by playing it out with Stockfish (see the tests): the win is a win, the draw is a draw. */
export const ENDGAMES: Endgame[] = [
  { id: "kq-k", title: "Queen and king checkmate", level: "Beginner", fen: "8/8/8/4k3/8/8/8/3QK3 w - - 0 1", player: "white", goal: "win",
    blurb: "The basic mate. Corner the king, then bring your own king up.", tip: "Keep the queen a knight's move away from the king so it can't be taken, push the king to the edge, then walk your king over. Watch out for stalemate!" },
  { id: "kr-k", title: "Rook and king checkmate", level: "Beginner", fen: "8/8/8/4k3/8/8/8/R3K3 w - - 0 1", player: "white", goal: "win",
    blurb: "Slower than the queen, but just as certain.", tip: "Use the rook to cut off the king along a file or rank, bring your king to oppose theirs, then check with the rook on the edge." },
  { id: "kbb-k", title: "Two bishops checkmate", level: "Improver", fen: "8/8/8/4k3/8/8/8/2B1KB2 w - - 0 1", player: "white", goal: "win",
    blurb: "Two bishops side by side form a wall the king cannot cross.", tip: "Keep the bishops next to each other, shrink the area the king can use, and drive it into a corner of the bishops' colour." },
  { id: "kp-win", title: "King in front of the pawn", level: "Beginner", fen: "4k3/8/4K3/4P3/8/8/8/8 w - - 0 1", player: "white", goal: "win",
    blurb: "Your king leads the way and the pawn follows.", tip: "With your king in front of your pawn on the 6th rank you win. Push the pawn only when it keeps the opposition, and watch out for stalemate." },
  { id: "kp-keysquares", title: "Pawn and king: the key squares", level: "Improver", fen: "8/8/8/8/3k4/8/4PK2/8 w - - 0 1", player: "white", goal: "win",
    blurb: "Get your king ahead of the pawn and escort it home.", tip: "Your king must reach a square in front of the pawn. Use your king to gain space, then support every pawn push." },
  { id: "kp-opposition", title: "Hold the draw: the opposition", level: "Improver", fen: "8/8/8/4k3/8/4K3/4P3/8 w - - 0 1", player: "black", goal: "draw",
    blurb: "Your king blocks the pawn. Keep it there and the game is a draw.", tip: "Stay directly in front of the pawn with the opposition (one empty square between the kings). When White steps aside, step aside too." },
  { id: "rook-pawn", title: "Rook pawn, king far away", level: "Improver", fen: "8/8/8/8/8/7k/P7/K7 w - - 0 1", player: "white", goal: "win",
    blurb: "A rook pawn is tricky, but here the black king is too far to stop it.", tip: "Just run: push the pawn all the way and promote. Keep your king close in case the black king comes back." },
  { id: "lucena", title: "The Lucena position: build a bridge", level: "Intermediate", fen: "1K1k4/1P6/8/8/8/8/r7/2R5 w - - 0 1", player: "white", goal: "win",
    blurb: "The most famous rook ending. You are one step from promotion.", tip: "Your king is stuck in front of the pawn. Bring your rook to the 4th rank (the bridge), walk your king out and let the rook shield it from checks." },
  { id: "philidor", title: "The Philidor position: hold the draw", level: "Intermediate", fen: "4k3/8/r7/4P3/4K3/8/8/4R3 b - - 0 1", player: "black", goal: "draw",
    blurb: "Defend the rook ending by keeping your rook on the 6th rank.", tip: "Keep your rook on the 6th rank to stop the white king advancing. When the pawn finally moves to the 6th, switch your rook to the back rank and check from behind." },
];
export const getEndgame = (id: string): Endgame | undefined => ENDGAMES.find((e) => e.id === id);
