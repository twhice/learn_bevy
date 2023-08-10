use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // .insert_resource(ClearColor(Color::WHITE)) 灰色也挺好看
        .insert_resource(FixedTime::new_from_secs(1.0 / 60.0))
        .init_resource::<Board>()
        .init_resource::<Round>()
        .init_resource::<NextPiecePosition>()
        .add_event::<PieceDown>()
        .add_state::<GameState>()
        .add_systems(Startup, setup)
        .add_systems(
            FixedUpdate,
            (
                (put_piece, piece_down.after(put_piece))
                    .run_if(in_state(GameState::InGame))
                    .after(restart),
                restart.run_if(in_state(GameState::GameOver)),
            ),
        )
        .add_systems(Update, (bevy::window::close_on_esc, update_mouse_location))
        .run()
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, States)]
pub enum GameState {
    InGame,
    GameOver,
}

impl Default for GameState {
    fn default() -> Self {
        Self::InGame
    }
}

#[derive(Debug, Component)]
pub struct GameOver;

/// 棋盘上线的颜色
const BOARD_LINE_COLOR: Color = Color::rgb(0.5, 0.5, 0.5);
/// 棋盘上线的宽度
const BOARD_LINE_WIDTH: f32 = 10.0;
/// 棋盘
#[derive(Resource, Default, Debug, Clone)]
pub struct Board {
    /// 对应棋子对应的Entity
    slots: [[Option<PieceColor>; 15]; 15],
}

impl Board {
    pub fn piece_position(piece_position: (usize, usize)) -> Vec3 {
        // 棋盘大小为15*15
        let dx = piece_position.0 as f32 - 7.0;
        let dy = 7.0 - piece_position.1 as f32;
        Vec3::new(
            CHESS_PIECE_SIZE.x * dx,
            CHESS_PIECE_SIZE.y * dy,
            CHESS_PIECE_SIZE.z,
        )
    }
}

/// 棋子的大小
const CHESS_PIECE_SIZE: Vec3 = Vec3::new(30.0, 30.0, 30.0);
/// 棋子
#[derive(Component, Debug, Clone, Copy)]
pub struct Piece;

#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct Round(PieceColor);

/// 棋子的颜色
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PieceColor {
    /// 黑色
    White,
    /// 白色
    Black,
}

impl std::fmt::Display for PieceColor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PieceColor::White => write!(f, "白棋"),
            PieceColor::Black => write!(f, "黑棋"),
        }
    }
}

/// 黑棋先手
impl Default for PieceColor {
    fn default() -> Self {
        Self::Black
    }
}

impl PieceColor {
    const fn to_color(self) -> Color {
        match self {
            PieceColor::White => Color::WHITE,
            PieceColor::Black => Color::BLACK,
        }
    }
}

#[derive(Resource, Default, Debug, Clone)]
pub struct NextPiecePosition(Option<Vec2>);

/// 绘制棋盘
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    assets: Res<AssetServer>,
) {
    commands.spawn(Camera2dBundle::default());
    // 横着的线的尺寸
    let horzontal_line_size = Vec2::new(
        14.0 * CHESS_PIECE_SIZE.x + BOARD_LINE_WIDTH,
        BOARD_LINE_WIDTH,
    );

    // 竖着的线的尺寸
    let vertical_line_size = Vec2::new(
        BOARD_LINE_WIDTH,
        14.0 * CHESS_PIECE_SIZE.y + BOARD_LINE_WIDTH,
    );

    // 棋盘上的线
    for i in -7..=7 {
        let i = i as f32;
        // 水平线条
        commands.spawn(ColorMesh2dBundle {
            mesh: meshes
                .add(shape::Quad::new(horzontal_line_size).into())
                .into(),
            material: materials.add(ColorMaterial::from(BOARD_LINE_COLOR)),
            transform: Transform::from_translation(Vec3::new(0.0, i * CHESS_PIECE_SIZE.y, 0.0)),
            ..Default::default()
        });
        commands.spawn(ColorMesh2dBundle {
            mesh: meshes
                .add(shape::Quad::new(vertical_line_size).into())
                .into(),
            material: materials.add(ColorMaterial::from(BOARD_LINE_COLOR)),
            transform: Transform::from_translation(Vec3::new(i * CHESS_PIECE_SIZE.y, 0.0, 0.0)),
            ..Default::default()
        });
    }

    // 提前准备胜利的字样
    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|parent| {
            parent.spawn((
                GameOver,
                TextBundle {
                    text: Text::from_section(
                        "肮脏的黑客！",
                        TextStyle {
                            font_size: 50.0,
                            color: Color::ORANGE,
                            font: assets.load("LXGWWenKaiMono-Regular.ttf"),
                        },
                    )
                    .with_alignment(TextAlignment::Center),
                    visibility: Visibility::Hidden,

                    ..Default::default()
                },
            ));
        });
}

/// 更新鼠标位置
fn update_mouse_location(
    mouse_button: Res<Input<MouseButton>>,
    mut next_piece_position: ResMut<NextPiecePosition>,
    mut cursor: EventReader<CursorMoved>,
    mut newest_mouse_position: Local<Vec2>,
) {
    if let Some(last) = cursor.into_iter().last() {
        *newest_mouse_position = last.position;
    }
    if mouse_button.just_pressed(MouseButton::Left) {
        next_piece_position.0 = Some(*newest_mouse_position);
    }
}

#[derive(Debug, Clone, Copy, Event)]
struct PieceDown((usize, usize), PieceColor);

/// 尝试放下棋子
fn put_piece(
    window: Query<&Window>,
    mut round: ResMut<Round>,
    mut board: ResMut<Board>,
    mut cursor_position: ResMut<NextPiecePosition>,
    mut piece_down: EventWriter<PieceDown>,
) {
    let Some(mut piece_position) = cursor_position.0.take() else {
        return;
    };

    // 转换得到棋盘上的坐标
    let window = window.get_single().unwrap();
    piece_position.x -= window.width() / 2.0;
    piece_position.y -= window.height() / 2.0;
    piece_position.x /= CHESS_PIECE_SIZE.x;
    piece_position.y /= CHESS_PIECE_SIZE.y;
    piece_position.x = piece_position.x.round();
    piece_position.y = piece_position.y.round();
    if piece_position.x < -7.0
        || piece_position.x > 7.0
        || piece_position.y < -7.0
        || piece_position.y > 7.0
    {
        return;
    }
    let (piece_x, piece_y) = (
        (piece_position.x + 7.0) as usize,
        (piece_position.y + 7.0) as usize,
    );

    // 没有棋子时才可以放置棋子
    if board.slots[piece_x][piece_y].is_none() {
        piece_down.send(PieceDown((piece_x, piece_y), round.0));
        // 放置棋子

        board.slots[piece_x][piece_y] = Some(round.0);
        round.0 = match round.0 {
            PieceColor::White => PieceColor::Black,
            PieceColor::Black => PieceColor::White,
        }
    }
}

/// 进行放下棋子，以及进行胜利检测
fn piece_down(
    board: Res<Board>,
    mut piece_down: EventReader<PieceDown>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut state: ResMut<NextState<GameState>>,
    mut text: Query<(&mut Text, &mut Visibility), With<GameOver>>,
) {
    for PieceDown((piece_x, piece_y), piece_color) in piece_down.into_iter().copied() {
        commands.spawn((
            Piece,
            ColorMesh2dBundle {
                mesh: meshes
                    .add(
                        shape::Circle::new(CHESS_PIECE_SIZE.x.max(CHESS_PIECE_SIZE.y) / 2.0).into(),
                    )
                    .into(),
                material: materials.add(ColorMaterial::from(piece_color.to_color())),
                transform: Transform::from_translation(Board::piece_position((piece_x, piece_y))),
                ..Default::default()
            },
        ));
        // 查看是否胜利
        // 向八个方向延申
        let fronts = [
            (1, 0),
            (0, 1),
            (-1, 0),
            (0, -1),
            (1, 1),
            (1, -1),
            (-1, 1),
            (-1, -1),
        ];
        for front in fronts {
            let (mut piece_x, mut piece_y) = (piece_x as isize, piece_y as isize);
            let if_win = 'gameover: {
                for _ in 0..4 {
                    piece_x += front.0;
                    piece_y += front.1;
                    if piece_x.is_negative()
                        || piece_y.is_negative()
                        || !board.slots.get(piece_x as usize).is_some_and(|s| {
                            s.get(piece_y as usize)
                                .is_some_and(|optc| optc.is_some_and(|c| c == piece_color))
                        })
                    {
                        break 'gameover false;
                    }
                }
                true
            };
            if if_win {
                state.set(GameState::GameOver);
                let (mut text, mut vis) = text.single_mut();
                *vis = Visibility::Visible;
                text.sections[0].value = format!("{piece_color}胜！");
                // 虽然不会发生一帧两棋...说不定呢
                break;
            }
        }
    }
}

fn restart(
    mut commands: Commands,
    pieces: Query<Entity, With<Piece>>,
    mut text: Query<&mut Visibility, With<GameOver>>,
    mut cursor_position: ResMut<NextPiecePosition>,
    mut state: ResMut<NextState<GameState>>,
    mut board: ResMut<Board>,
) {
    if cursor_position.0.take().is_none() {
        return;
    }

    for entity in pieces.iter() {
        commands.entity(entity).despawn();
    }
    *board = Board::default();
    let mut vis = text.single_mut();
    *vis = Visibility::Hidden;
    state.set(GameState::InGame);

    // 删除所有entity
}

fn foo() {
    let x = 0;
    match x {
        x @ 1..=3 => println!(""),
        _ => println!(""),
    }
}
