use roxmltree::Node;

pub trait MavenParser {
    fn get_latest(&self) -> Option<(String, u64)>;
    fn get_child(&self, name: &str) -> Option<Node<'_, '_>>;
}

impl MavenParser for Node<'_, '_> {
    fn get_child(&self, name: &str) -> Option<Node<'_, '_>> {
        self.descendants()
            .find(|&descendent| descendent.tag_name().name() == name)
    }

    fn get_latest(&self) -> Option<(String, u64)> {
        let metadata = self.get_child("metadata")?;
        let versioning = metadata.get_child("versioning")?;
        Some((
            versioning.get_child("latest")?.text()?.to_owned(),
            versioning
                .get_child("lastUpdated")?
                .text()?
                .to_owned()
                .parse::<u64>()
                .ok()?,
        ))
    }
}
