export type Category = "Games" | "Computer" | "Puzzles" | "Learning" | "Community" | "Tournaments";

export type Achievement = { key: string; name: string; icon: string; category: Category; desc: string };

/** Badge definitions. The conditions live in the database (`check_achievements()`); a test keeps the two lists in step. */
export const ACHIEVEMENTS: Achievement[] = [
  { key: "first_game", name: "First move", icon: "♟️", category: "Games", desc: "Finish your first game against a classmate." },
  { key: "first_win", name: "First blood", icon: "🏆", category: "Games", desc: "Win a game against a classmate." },
  { key: "games_25", name: "Regular", icon: "🎲", category: "Games", desc: "Finish 25 games." },
  { key: "wins_10", name: "On a roll", icon: "🔥", category: "Games", desc: "Win 10 games." },
  { key: "wins_50", name: "Champion", icon: "👑", category: "Games", desc: "Win 50 games." },
  { key: "daily_win", name: "Patient player", icon: "📬", category: "Games", desc: "Win a daily (correspondence) game." },
  { key: "rated_1600", name: "Rising", icon: "📈", category: "Games", desc: "Reach a rating of 1600 (at least 10 games)." },
  { key: "rated_1800", name: "Strong player", icon: "💪", category: "Games", desc: "Reach a rating of 1800." },
  { key: "rated_2000", name: "Expert", icon: "🌟", category: "Games", desc: "Reach a rating of 2000." },
  { key: "bot_first_win", name: "Bot basher", icon: "🤖", category: "Computer", desc: "Beat a bot in a game of at least 5 moves each." },
  { key: "bot_dermann", name: "Der Mann", icon: "🧔", category: "Computer", desc: "Beat Der Mann. It should not be hard." },
  { key: "bot_beat_1000", name: "Giant killer", icon: "🗡️", category: "Computer", desc: "Beat a bot rated 1000 or more." },
  { key: "bot_beat_1500", name: "Club crusher", icon: "⚔️", category: "Computer", desc: "Beat a bot rated 1500 or more." },
  { key: "bot_beat_2000", name: "Machine whisperer", icon: "🧠", category: "Computer", desc: "Beat a bot rated 2000 or more." },
  { key: "bot_beat_2500", name: "Grandmaster slayer", icon: "💥", category: "Computer", desc: "Beat a bot rated 2500 or more." },
  { key: "bot_ruedenrache", name: "Revenge denied", icon: "🐺", category: "Computer", desc: "Beat Der Mann (Rüdenrache), the 4000 bot." },
  { key: "puzzles_10", name: "Puzzler", icon: "🧩", category: "Puzzles", desc: "Solve 10 puzzles." },
  { key: "puzzles_100", name: "Puzzle fan", icon: "🎯", category: "Puzzles", desc: "Solve 100 puzzles." },
  { key: "puzzles_500", name: "Puzzle master", icon: "🏅", category: "Puzzles", desc: "Solve 500 puzzles." },
  { key: "puzzle_1800", name: "Sharp eyes", icon: "👁️", category: "Puzzles", desc: "Reach a puzzle rating of 1800." },
  { key: "rush_10", name: "Rusher", icon: "⏱️", category: "Puzzles", desc: "Score 10 in Puzzle Rush." },
  { key: "rush_25", name: "Speed demon", icon: "⚡", category: "Puzzles", desc: "Score 25 in Puzzle Rush." },
  { key: "lesson_first", name: "Student", icon: "📘", category: "Learning", desc: "Solve your first lesson exercise." },
  { key: "basics_done", name: "Knows the moves", icon: "♞", category: "Learning", desc: "Finish the \"How the pieces move\" lesson." },
  { key: "lessons_50", name: "Bookworm", icon: "📚", category: "Learning", desc: "Solve 50 lesson exercises." },
  { key: "endgame_first", name: "Endgame apprentice", icon: "🏁", category: "Learning", desc: "Master your first endgame drill." },
  { key: "endgames_all", name: "Endgame expert", icon: "🎓", category: "Learning", desc: "Master all nine endgame drills." },
  { key: "first_friend", name: "Friendly", icon: "🤝", category: "Community", desc: "Make your first friend." },
  { key: "friends_5", name: "Popular", icon: "🎉", category: "Community", desc: "Have 5 friends." },
  { key: "club_member", name: "Joiner", icon: "🏰", category: "Community", desc: "Join a club." },
  { key: "club_owner", name: "Founder", icon: "🚩", category: "Community", desc: "Start a club." },
  { key: "forum_first", name: "Voice", icon: "💬", category: "Community", desc: "Write your first forum post." },
  { key: "forum_20", name: "Regular poster", icon: "📝", category: "Community", desc: "Write 20 forum posts." },
  { key: "tournament_played", name: "Competitor", icon: "🏟️", category: "Tournaments", desc: "Play a game in a tournament." },
  { key: "tournament_podium", name: "On the podium", icon: "🥉", category: "Tournaments", desc: "Finish in the top three of a tournament (3+ players)." },
  { key: "tournament_winner", name: "Tournament winner", icon: "🥇", category: "Tournaments", desc: "Win a tournament (3+ players)." },
];

export const CATEGORIES: Category[] = ["Games", "Computer", "Puzzles", "Learning", "Community", "Tournaments"];
export const achievementOf = (key: string): Achievement | undefined => ACHIEVEMENTS.find((a) => a.key === key);
