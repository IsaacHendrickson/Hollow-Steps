//! The table under the picture (one row per placed thing) and the legend.
use super::scene::Thing;

/// A number as the room files write it: whole when it is whole, else one decimal.
fn number(value: f32) -> String {
    if (value - value.round()).abs() < 0.05 {
        format!("{:.0}", value.round())
    } else {
        format!("{value:.1}")
    }
}

/// One row per thing: mark, kind, the top-left corner and size in px, and the details.
pub(super) fn table(things: &[Thing]) -> Vec<String> {
    let rows: Vec<[String; 7]> = things
        .iter()
        .map(|thing| {
            [
                thing.mark.to_string(),
                thing.kind.to_owned(),
                number(thing.rect.x),
                number(thing.rect.y),
                number(thing.rect.w),
                number(thing.rect.h),
                thing.detail.clone(),
            ]
        })
        .collect();
    let titles = ["mark", "kind", "x", "y", "w", "h", "details"];
    let mut widths = titles.map(str::len);
    for row in &rows {
        for (width, cell) in widths.iter_mut().zip(row) {
            *width = (*width).max(cell.len());
        }
    }
    let line = |cells: &[String]| {
        let numbers = |i: usize| format!("{:>w$}", cells[i], w = widths[i]);
        format!(
            "{:<w0$}  {:<w1$}  {}  {}  {}  {}  {}",
            cells[0],
            cells[1],
            numbers(2),
            numbers(3),
            numbers(4),
            numbers(5),
            cells[6],
            w0 = widths[0],
            w1 = widths[1],
        )
        .trim_end()
        .to_owned()
    };
    let mut lines = vec![line(&titles.map(str::to_owned))];
    lines.extend(rows.iter().map(|row| line(row)));
    lines
}

/// The legend, printed under every preview.
pub(super) const LEGEND: &[&str] = &[
    "Legend (a cell holds the most important mark that lands in it; the table lists everything):",
    "  .  air                          #  terrain, at least half the cell",
    "  =  thin terrain, flat           |  thin terrain, upright          +  thin terrain, a corner or sliver",
    "  @  spawn (player body)          1-9, a-z  exits in file order (a door is an exit too), see the table",
    "  ^  hazard                       %  breakable wall                 /  switch        ~  barrier",
    "  $  amber shard   D  Dash   J  Double Jump   K  key   C  charm   *  coin   S  pickup slot",
    "  E  enemy body    <-->  its patrol range (body centers; the ends show over the body)   B  boss body",
    "Exits, hazards and puzzle pieces mark every cell they touch; bodies and pickups the cells they mostly cover.",
    "Terrain below the floor line is drawn as #. With none, the bottom edge of the picture is the floor.",
    "Table x, y, w, h: top-left corner and size in px, in room coordinates (the numbers in the room file).",
    "Backgrounds and the look of the terrain are not drawn.",
];
