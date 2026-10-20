use crate::Text;

use super::super::ChangeIterator;
use super::super::Composable;
use super::super::ChangeDesc;
use super::super::ChangeSet;
use super::super::ChangeMap;
use super::super::Section;
use super::super::Mapable;
use super::super::Change;
use super::super::Range;


impl Mapable for ChangeSet {}
impl Composable for ChangeSet {
    type Text = Text;

    fn sections(&self) -> &[Section] { &self.description.sections() }
    fn len_old(&self) -> usize { self.description.len_old() }
    fn len_new(&self) -> usize { self.description.len_new() }
    fn is_empty(&self) -> bool { self.description.is_empty() }
    fn text_slice(&self, section_idx: usize, offset: usize, len: usize) -> Self::Text {
        let text = self.insertions.get(section_idx).expect("that insertions.len == sections.len");
        text.slice(offset, Some(offset + len))
    }

    fn text_at(&self, section_idx: usize) -> Self::Text {
        self.insertions[section_idx].clone()
    }

    fn from_composed(sections: Vec<Section>, texts: Vec<Self::Text>) -> Self {
        Self { description: ChangeDesc { sections }, insertions: texts }
    }
}

impl ChangeIterator for ChangeSet {
    type Item = Change<Range>;

    fn get_change_iterator_item(&self, sections: Range, map: ChangeMap<Range>) -> Self::Item {
        let mut text = Text::new();
        for idx in sections.from..sections.to {
            text = text.append(self.text_at(idx));
        }
        Change { map, text }
    }
}
