//! The worked examples of every isle, by title.

/// Each isle's examples, by title: (text with `▮`, keys).
pub(super) static EXAMPLES: &[(&str, &[(&str, &str)])] = &[
    ("First Steps", &[("▮cat\ndog", "lljh"), ("hj▮kl", "hhll")]),
    (
        "Counting Sheep",
        &[("▮one two three", "4l"), ("▮a\nb\nc\nd", "3j")],
    ),
    (
        "Word Hopping",
        &[
            ("▮the quick brown fox", "ww"),
            ("the quick br▮own", "b"),
            ("▮the quick brown", "ee"),
        ],
    ),
    (
        "Punctuation Pass",
        &[("▮a.b.c d.e.f", "W"), ("▮x(1,2) y", "E"), ("x(1,2) ▮y", "B")],
    ),
    (
        "Edge of the World",
        &[
            ("   ind▮ented line", "0"),
            ("▮   indented", "^"),
            ("   ▮indented line", "$"),
        ],
    ),
    (
        "Long Jumps",
        &[
            ("▮line 1\nline 2\nline 3\nline 4", "G"),
            ("line 1\nline 2\n▮line 3", "gg"),
            ("▮a\nb\nc\nd", "3G"),
        ],
    ),
    (
        "Seek and Find",
        &[
            ("▮go fetch, the ball", "f,"),
            ("▮a,b,c,d", "f,;;"),
            ("▮call(x)", "t("),
            ("a,b,c▮,d", "F,"),
        ],
    ),
    ("Bug Hunt", &[("▮the bugs nibble", "3fb"), ("▮a.b.c.d", "2f.")]),
    (
        "What's in a Name",
        &[
            ("▮x = x + 1", "*ciwy<Esc>n."),
            ("let ▮a = a;", ":s/\\<a\\>/b/g<CR>"),
        ],
    ),
    (
        "Broken Config",
        &[("▮{'a'}", "f'r\";r\""), ("x: ▮true", "ciwfalse<Esc>")],
    ),
    ("Release Notes", &[("▮a\nb\nc", "ddGp"), ("▮a\nb\nc", ":1m$<CR>")]),
    ("Import Duty", &[("▮b\na\nb", ":sort u<CR>")]),
    (
        "Fading Lights",
        &[("▮a long line of text", "$"), ("a\nb\n▮c", "-")],
    ),
    ("Night Watch", &[("▮a\nb\nc\nd", "3j"), ("▮abcdefgh", "6l")]),
    ("Tide Clock", &[("▮abcdef", "4|"), ("▮abc\ndef", "$j")]),
    ("Name Swap", &[("▮Doe, Jane", ":s/\\v(.*), (.*)/\\2 \\1<CR>")]),
    ("Count Off", &[("▮1. a\nb", "0y3lj0P0<C-a>")]),
    (
        "Tag Soup",
        &[("▮<b>x</b>", ":s/<\\/\\=b>//g<CR>"), ("▮a\nb", ":1,2><CR>")],
    ),
    ("No Arrows", &[("▮one two\nthree", "+"), ("▮find the x", "fx")]),
    ("Thin Ice", &[("▮keep\nfix teh\nkeep", ":v/keep/s/teh/the<CR>")]),
    (
        "No Delete",
        &[
            ("▮the the end", ":s/\\v<(\\w+) \\1>/\\1/<CR>"),
            ("▮bad word", "cwgood<Esc>"),
        ],
    ),
    ("Every Key Counts", &[("▮short\nmuch longer", "$j")]),
    (
        "Paragraph Isles",
        &[("▮one\ntwo\n\nthree\n\nfour", "}}"), ("one\n\ntwo\nth▮ree", "{")],
    ),
    (
        "Mirror Matching",
        &[("▮if (a(b)) x", "%"), ("f(a, [1, 2▮])", "%")],
    ),
    (
        "Search Party",
        &[("▮cat dog cat bird", "/cat<CR>n"), ("▮dog cat dog", "*")],
    ),
    ("Infestation", &[("▮a bug, a bug", "/bug<CR>n")]),
    (
        "Looking Back",
        &[
            ("cat\ndog\n▮cat", "?cat<CR>"),
            ("▮a\nb\nc\nd", "G<C-o><Tab>"),
            ("a\n▮bee\nc", "<C-g>"),
        ],
    ),
    (
        "Chomp",
        &[
            ("hel▮llo", "x"),
            ("▮bat", "rc"),
            ("▮oops", "xu"),
            ("ab▮c def", "xxU"),
            ("▮hello", "~~"),
        ],
    ),
    (
        "Demolition",
        &[
            ("▮delete me keep", "dw"),
            ("keep ▮the rest", "D"),
            ("▮line one\nline two", "dd"),
            ("▮a,b,c", "dt,"),
            ("▮a\nb\nc\nd", "2dd"),
            ("▮a b c d", "d3w"),
        ],
    ),
    (
        "Change of Heart",
        &[
            ("▮hello world", "cwbye<Esc>"),
            ("x = ▮old;", "Cnew;<Esc>"),
            ("▮abc", "sX<Esc>"),
        ],
    ),
    (
        "Insert Coin",
        &[("▮list", "Imy <Esc>"), ("▮list", "A:<Esc>"), ("▮a\nc", "ob<Esc>")],
    ),
    ("Copycat", &[("▮hip hooray", "ywP"), ("▮==", "yy2p")]),
    ("Time Travel", &[("▮x\ny\nz", "A;<Esc>j.j."), ("▮abc", "xu<C-r>")]),
    (
        "Inside Out",
        &[
            ("say \"▮hi there\"", "ci\"yo<Esc>"),
            ("f(▮a, b)", "di("),
            ("a ▮word here", "daw"),
        ],
    ),
    (
        "Highlights",
        &[("▮shout this", "veU"), ("▮a\nb\nc", "Vjd"), ("▮one\ntwo", "J")],
    ),
    (
        "Ex Marks the Spot",
        &[
            ("▮foo is foo", ":s/foo/bar/<CR>"),
            ("▮foo\nfoo foo", ":%s/foo/bar/g<CR>"),
            ("▮a\nb\nc", ":3<CR>"),
        ],
    ),
    (
        "Case Closed",
        &[
            ("▮Gold gold GOLD", ":set ic<CR>/gold<CR>n"),
            ("▮a gold GOLD", "/GOLD\\c<CR>"),
            ("▮cat, a cat", ":set hls<CR>/cat<CR>"),
        ],
    ),
    ("Ask First", &[("▮cat cat cat", ":s/cat/dog/gc<CR>yny")]),
    (
        "Scrolls",
        &[
            ("▮# shelf", ":r notes.txt<CR>"),
            ("▮x", ":!ls<CR>"),
            ("▮x", ":r t<Tab><CR>"),
        ],
    ),
    (
        "Copy Desk",
        &[
            ("▮a\nb\nc", "jVj:w part.txt<CR>:!cat part.txt<CR>"),
            ("▮files:", ":r !ls<CR>"),
        ],
    ),
    (
        "The Refactor",
        &[("▮fn old() {}", "wcwnew<Esc>"), ("▮i32 + i32", ":s/32/64/g<CR>")],
    ),
    (
        "Two Scrolls",
        &[
            ("▮eggs", ":e todo.txt<CR>yy:e #<CR>p"),
            ("▮x", ":e todo.txt<CR>Ax<Esc>:w<CR>"),
        ],
    ),
    (
        "Named Vaults",
        &[
            ("▮one two", "\"ayw:reg<CR>"),
            ("▮lamp\ncrate\nvault:", "\"addj\"ap"),
            ("▮keep\njunk", "yyj\"_ddp"),
        ],
    ),
    (
        "Waymarks",
        &[
            ("▮start\nmiddle\nend", "maG`a"),
            ("keep\n▮cut\ncut\nkeep", "majd'a"),
        ],
    ),
    (
        "Tally Marks",
        &[
            ("x = ▮7", "<C-a>"),
            ("▮year 1999", "27<C-a>"),
            ("▮hp: 10", "3<C-x>"),
        ],
    ),
    (
        "Wind-up Golem",
        &[("▮a\nb\nc", "qaA;<Esc>jq@a"), ("▮1\n2\n3", "qaI-<Esc>jq@a@@")],
    ),
    ("Assembly Line", &[("▮item 1", "qayyp<C-a>q2@a")]),
    (
        "Terraces",
        &[("▮code", ">>"), ("▮a\nb", ">j"), ("        ▮x", "<<")],
    ),
    (
        "Shouting Match",
        &[
            ("▮hello world", "gUiw"),
            ("▮Mixed Case", "guu"),
            ("▮abc def", "g~w"),
        ],
    ),
    (
        "Overwrite",
        &[("| ▮????? |", "Rhello<Esc>"), ("▮abcd", "RXY<BS><Esc>")],
    ),
    (
        "Ice Columns",
        &[
            ("▮red\ngreen\nblue", "<C-v>jjI- <Esc>"),
            ("▮a\nbb\nccc", "<C-v>jj$A;<Esc>"),
            ("▮# x\n# y", "<C-v>jld"),
        ],
    ),
    (
        "Next Match",
        &[
            ("▮foo bar foo baz", "/foo<CR>cgnqux<Esc>."),
            ("▮tmp tmpdir tmp", "*"),
        ],
    ),
    (
        "Tag, You're It",
        &[
            ("<b>▮bold</b>", "cithi<Esc>"),
            ("<p><b>▮x</b> y</p>", "dat"),
            ("▮a\nb\n\nc", "dap"),
        ],
    ),
    (
        "Global Decree",
        &[
            ("▮keep\n# drop\nkeep\n# drop", ":g/#/d<CR>"),
            ("▮x1\ny\nx2", ":v/x/d<CR>"),
            ("▮b\nc\na", ":sort<CR>"),
        ],
    ),
    (
        "Moving Day",
        &[
            ("▮a\nb\nc\nd", ":3,4m0<CR>"),
            ("▮a\nb\nc", ":$t0<CR>"),
            ("▮a\nb\nc", "Vj:d<CR>"),
        ],
    ),
    (
        "Normal Order",
        &[
            ("▮fn a\n// x\nfn b", ":g/fn/norm A;<CR>"),
            ("▮a\nb", ":%norm I- <CR>"),
        ],
    ),
    (
        "The Grand Refactor",
        &[("▮a=1\nb=2", ":%s/=/ = /<CR>"), ("▮x\ny", "qaIconst <Esc>jq@a")],
    ),
];
