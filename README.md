##sound font##
github에는 100mb까지만 가능하여 sound font를 추가하는 작업은 따로 파일로 주고받아야 할듯

##2021.04.09##
F1키를 누르면 노트의 색이 변하도록 ./playing_scene/notes.rs와 ./playing_scene/mod.rs 수정

./playing_scene/notes.rs 에 recolor()라는 함수를 추가.. 기존에 있던 resize()를 그대로 가져오며 color 부분만 수정함

./playing_scene/mod.rs 에 F1버튼 누르는 이벤트 추가

cargo run을 하여 노트가 생성되면, 즉 config.rs가 호출되면 setting.ron 이 생성되고, 이 ron 파일을 직접 바꿔야 config에서 설정하는 default가 적용되는 것을 확인

혹은 ron 파일을 삭제한 후 config.rs 를 수정하면 default가 적용됨
