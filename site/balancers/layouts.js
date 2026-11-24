// Tile layouts from Factorio-SAT, books/blue_narrow.txt (blob bb6b24caf70662973450417a8796d036677e382d).
// https://github.com/R-O-C-K-E-T/Factorio-SAT/blob/main/books/blue_narrow.txt
// Distributed under GPL-3.0. See layouts.LICENSE.txt.
// ^ > v < are belt directions. i/o mark tunnel ends. S/s are the two tiles of a splitter.
export const layouts = {
  2: [
    "> S >",
    "> s >",
  ],
  4: [
    "> S > > v > > > S >",
    "> s S i> v ^ < o> s >",
    "> S s i> > S ^ o> S >",
    "> s > > > s > > s >",
  ],
  8: [
    "> S > v > v > > > > S >",
    "> s i> v o^ > ^ o> > S s >",
    "> S i> > S > > v o> s S >",
    "> s > > s v . iv > > s >",
    "> S > v i^ > i> > ^ o> S >",
    "> s i> v ^ < > ^ o> S s >",
    "> S i> > S ^ ^ ov o> s S >",
    "> s > > s > ^ > > > s >",
  ],
};
