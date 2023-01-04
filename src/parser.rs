const X: usize = 8;
const Y: usize = 10;

pub fn parse_ascii_to_binary(map: String) -> Vec<Vec<i32>> {
    let mut arr: Vec<Vec<i32>> = vec![vec![0; Y]; X];

    for (i, x) in map.lines().enumerate() {
        let x = x.trim();
        for (j, y) in x.chars().enumerate() {
            match y {
                '.' => arr[i][j] = 1,
                '#' => arr[i][j] = 0,
                _ => unreachable!(),
            }
        }
    }
    arr
}

pub fn parse_binary_to_ascii(binary_map: Vec<Vec<i32>>) -> String {
    let mut str_map = String::new();

    for (i, g) in binary_map.iter().enumerate() {
        for (j, _) in g.iter().enumerate() {
            match binary_map[i][j] {
                1 => str_map.push('.'),
                0 => str_map.push('#'),
                _ => unreachable!(),
            }
        }
        str_map.push('\n');
    }
    str_map
}
