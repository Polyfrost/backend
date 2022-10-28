use roxmltree::Node;

pub trait MavenParser {
    fn get_latest(&self) -> Option<String>;
    fn get_child(&self, name: &str) -> Option<Node<'_, '_>>;
}

impl MavenParser for Node<'_, '_> {
    fn get_child(&self, name: &str) -> Option<Node<'_, '_>> {
        self.descendants()
            .find(|&descendent| descendent.tag_name().name() == name)
    }

    fn get_latest(&self) -> Option<String> {
        Some(
            self.get_child("metadata")?
                .get_child("versioning")?
                .get_child("latest")?
                .text()?
                .to_owned(),
        )
    }
}
