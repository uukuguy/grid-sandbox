use octo_types::MemoryBlock;

const WORKING_MEMORY_BUDGET_CHARS: usize = 12_000;

pub struct ContextInjector;

impl ContextInjector {
    pub fn compile(blocks: &[MemoryBlock]) -> String {
        let mut sorted: Vec<&MemoryBlock> = blocks.iter().filter(|b| !b.value.is_empty()).collect();
        sorted.sort_by(|a, b| b.priority.cmp(&a.priority));

        let mut output = String::from("<working_memory>\n");
        let mut total_chars = 0;

        for block in sorted {
            let entry = format!(
                "<block kind=\"{}\" priority=\"{}\">{}</block>\n",
                block.id, block.priority, block.value
            );
            if total_chars + entry.len() > WORKING_MEMORY_BUDGET_CHARS {
                break;
            }
            total_chars += entry.len();
            output.push_str(&entry);
        }

        output.push_str("</working_memory>");
        output
    }
}
