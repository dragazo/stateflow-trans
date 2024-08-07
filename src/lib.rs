use netsblox_ast as ast;
use netsblox_ast::compact_str::{CompactString, ToCompactString, format_compact};

use graphviz_rust::dot_structures as dot;

use std::collections::{VecDeque, BTreeMap, BTreeSet};

#[cfg(test)]
mod test;

pub use graphviz_rust as graphviz;

macro_rules! count_exprs {
    () => { 0usize };
    ($head:expr $(,$tail:expr)* $(,)?) => { 1usize + count_exprs!($($tail),*) };
}
macro_rules! deque {
    ($($values:expr),* $(,)?) => {{
        #[allow(unused_mut)]
        let mut res = VecDeque::with_capacity(count_exprs!($($values),*));
        $(res.push_back($values);)*
        res
    }}
}

trait VecDequeUtil<T> {
    fn extend_front<I: Iterator<Item = T> + DoubleEndedIterator>(&mut self, iter: I);
}
impl<T> VecDequeUtil<T> for VecDeque<T> {
    fn extend_front<I: Iterator<Item = T> + DoubleEndedIterator>(&mut self, iter: I) {
        for val in iter.rev() {
            self.push_front(val);
        }
    }
}

fn punctuate(values: &[CompactString], sep: &str) -> Option<CompactString> {
    match values {
        [] => None,
        [h, t @ ..] => {
            let mut res = h.clone();
            for x in t {
                res.push_str(sep);
                res.push_str(x);
            }
            Some(res)
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum CompileError {
    ParseError(Box<ast::Error>),
    RoleCount { count: usize },
    UnknownRole { name: CompactString },
    UnsupportedBlock { state_machine: CompactString, state: CompactString, info: CompactString },
    NonTerminalTransition { state_machine: CompactString, state: CompactString },
    MultipleHandlers { state_machine: CompactString, state: CompactString },
    ComplexTransitionName { state_machine: CompactString, state: CompactString },
    VariadicBlocks { state_machine: CompactString, state: CompactString },
    ActionsOutsideTransition { state_machine: CompactString, state: CompactString },
    VariableOverlap { state_machines: (CompactString, CompactString), variable: CompactString },
}

#[derive(Debug, PartialEq, Eq)]
pub struct Project {
    pub name: CompactString,
    pub role: CompactString,
    pub state_machines: BTreeMap<CompactString, StateMachine>,
}
#[derive(Debug, PartialEq, Eq)]
pub struct StateMachine {
    pub variables: BTreeSet<CompactString>,
    pub states: BTreeMap<CompactString, State>,
    pub initial_state: Option<CompactString>,
    pub current_state: Option<CompactString>,
}
#[derive(Debug, PartialEq, Eq)]
pub struct State {
    pub transitions: VecDeque<Transition>,
}
#[derive(Debug, PartialEq, Eq)]
pub struct Transition {
    pub ordered_condition: Option<CompactString>,
    pub unordered_condition: Option<CompactString>,
    pub actions: VecDeque<CompactString>,
    pub new_state: CompactString,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Settings {
    pub omit_unknown_blocks: bool,
}
struct Context {
    variables: Vec<ast::VariableRef>,
    settings: Settings,
}

fn translate_value(state_machine: &str, state: &str, value: &ast::Value) -> Result<CompactString, CompileError> {
    Ok(match value {
        ast::Value::String(x) => x.clone(),
        ast::Value::Number(x) => x.to_compact_string(),
        ast::Value::Bool(x) => if *x { "true" } else { "false" }.into(),
        ast::Value::Constant(x) => match x {
            ast::Constant::E => core::f64::consts::E.to_compact_string(),
            ast::Constant::Pi => core::f64::consts::PI.to_compact_string(),
        }
        x => return Err(CompileError::UnsupportedBlock { state_machine: state_machine.into(), state: state.into(), info: format_compact!("{x:?}") }),
    })
}
fn translate_expr(state_machine: &str, state: &str, expr: &ast::Expr, context: &mut Context) -> Result<CompactString, CompileError> {
    fn extract_fixed_variadic<'a>(state_machine: &str, state: &str, values: &'a ast::Expr, context: &mut Context) -> Result<Vec<CompactString>, CompileError> {
        match &values.kind {
            ast::ExprKind::MakeList { values } => Ok(values.iter().map(|x| translate_expr(state_machine, state, x, context)).collect::<Result<_,_>>()?),
            _ => Err(CompileError::VariadicBlocks { state_machine: state_machine.into(), state: state.into() }),
        }
    }

    Ok(match &expr.kind {
        ast::ExprKind::Value(x) => translate_value(state_machine, state, x)?,
        ast::ExprKind::Variable { var } => {
            context.variables.push(var.clone());
            var.trans_name.clone()
        }
        ast::ExprKind::Sin { value } => format_compact!("sind({})", translate_expr(state_machine, state, &value, context)?),
        ast::ExprKind::Cos { value } => format_compact!("cosd({})", translate_expr(state_machine, state, &value, context)?),
        ast::ExprKind::Tan { value } => format_compact!("tand({})", translate_expr(state_machine, state, &value, context)?),
        ast::ExprKind::Asin { value } => format_compact!("asind({})", translate_expr(state_machine, state, &value, context)?),
        ast::ExprKind::Acos { value } => format_compact!("acosd({})", translate_expr(state_machine, state, &value, context)?),
        ast::ExprKind::Atan { value } => format_compact!("atand({})", translate_expr(state_machine, state, &value, context)?),
        ast::ExprKind::Sqrt { value } => format_compact!("sqrt({})", translate_expr(state_machine, state, &value, context)?),
        ast::ExprKind::Floor { value } => format_compact!("floor({})", translate_expr(state_machine, state, &value, context)?),
        ast::ExprKind::Ceil { value } => format_compact!("ceil({})", translate_expr(state_machine, state, &value, context)?),
        ast::ExprKind::Round { value } => format_compact!("round({})", translate_expr(state_machine, state, &value, context)?),
        ast::ExprKind::Sign { value } => format_compact!("sign({})", translate_expr(state_machine, state, &value, context)?),
        ast::ExprKind::Neg { value } => format_compact!("-{}", translate_expr(state_machine, state, &value, context)?),
        ast::ExprKind::Abs { value } => format_compact!("abs({})", translate_expr(state_machine, state, &value, context)?),
        ast::ExprKind::Sub { left, right } => format_compact!("({} - {})", translate_expr(state_machine, state, &left, context)?, translate_expr(state_machine, state, &right, context)?),
        ast::ExprKind::Div { left, right } => format_compact!("({} / {})", translate_expr(state_machine, state, &left, context)?, translate_expr(state_machine, state, &right, context)?),
        ast::ExprKind::Mod { left, right } => format_compact!("mod({}, {})", translate_expr(state_machine, state, &left, context)?, translate_expr(state_machine, state, &right, context)?),
        ast::ExprKind::Log { value, base } => format_compact!("(log({}) / log({}))", translate_expr(state_machine, state, &value, context)?, translate_expr(state_machine, state, &base, context)?),
        ast::ExprKind::Atan2 { y, x } => format_compact!("atan2d({}, {})", translate_expr(state_machine, state, &y, context)?, translate_expr(state_machine, state, &x, context)?),
        ast::ExprKind::Add { values } => punctuate(&extract_fixed_variadic(state_machine,state, values, context)?, " + ").map(|x| format_compact!("({x})")).unwrap_or_else(|| "0".into()),
        ast::ExprKind::Mul { values } => punctuate(&extract_fixed_variadic(state_machine,state, values, context)?, " * ").map(|x| format_compact!("({x})")).unwrap_or_else(|| "1".into()),
        ast::ExprKind::Pow { base, power } => format_compact!("({} ^ {})", translate_expr(state_machine, state, &base, context)?, translate_expr(state_machine, state, &power, context)?),
        ast::ExprKind::Eq { left, right } => format_compact!("{} == {}", translate_expr(state_machine, state, &left, context)?, translate_expr(state_machine, state, &right, context)?),
        ast::ExprKind::Neq { left, right } => format_compact!("{} ~= {}", translate_expr(state_machine, state, &left, context)?, translate_expr(state_machine, state, &right, context)?),
        ast::ExprKind::Greater { left, right } => format_compact!("{} > {}", translate_expr(state_machine, state, &left, context)?, translate_expr(state_machine, state, &right, context)?),
        ast::ExprKind::GreaterEq { left, right } => format_compact!("{} >= {}", translate_expr(state_machine, state, &left, context)?, translate_expr(state_machine, state, &right, context)?),
        ast::ExprKind::Less { left, right } => format_compact!("{} < {}", translate_expr(state_machine, state, &left, context)?, translate_expr(state_machine, state, &right, context)?),
        ast::ExprKind::LessEq { left, right } => format_compact!("{} <= {}", translate_expr(state_machine, state, &left, context)?, translate_expr(state_machine, state, &right, context)?),
        ast::ExprKind::And { left, right } => format_compact!("{} & {}", translate_expr(state_machine, state, &left, context)?, translate_expr(state_machine, state, &right, context)?),
        ast::ExprKind::Or { left, right } => format_compact!("({} | {})", translate_expr(state_machine, state, &left, context)?, translate_expr(state_machine, state, &right, context)?),
        ast::ExprKind::Not { value } => format_compact!("~({})", translate_expr(state_machine, state, &value, context)?),
        ast::ExprKind::Timer => "t".into(),
        x => match context.settings.omit_unknown_blocks {
            true => "?".into(),
            false => return Err(CompileError::UnsupportedBlock { state_machine: state_machine.into(), state: state.into(), info: format_compact!("{x:?}") }),
        }
    })
}
fn parse_actions(state_machine: &str, state: &str, stmt: &ast::Stmt, context: &mut Context) -> Result<Vec<CompactString>, CompileError> {
    Ok(match &stmt.kind {
        ast::StmtKind::Assign { var, value } => {
            context.variables.push(var.clone());
            vec![format_compact!("{} = {}", var.trans_name, translate_expr(state_machine, state, value, context)?)]
        }
        ast::StmtKind::ResetTimer => vec!["t = 0".into()],
        x => match context.settings.omit_unknown_blocks {
            true => vec!["?".into()],
            false => return Err(CompileError::UnsupportedBlock { state_machine: state_machine.into(), state: state.into(), info: format_compact!("{x:?}") }),
        }
    })
}
fn parse_transitions(state_machine: &str, state: &str, stmt: &ast::Stmt, terminal: bool, context: &mut Context) -> Result<Option<(VecDeque<Transition>, Option<CompactString>, bool)>, CompileError> {
    fn parse_transition_target(state_machine: &str, state: &str, expr: &ast::Expr, context: &mut Context) -> Result<VecDeque<Transition>, CompileError> {
        Ok(match &expr.kind {
            ast::ExprKind::Value(ast::Value::String(x)) => deque![Transition { ordered_condition: None, unordered_condition: None, actions: <_>::default(), new_state: x.clone() }],
            ast::ExprKind::Conditional { condition, then, otherwise } => {
                let condition = translate_expr(state_machine, state, condition, context)?;
                let mut then_transitions = parse_transition_target(state_machine, state, then, context)?;
                let mut otherwise_transitions = parse_transition_target(state_machine, state, otherwise, context)?;

                for transition in then_transitions.iter_mut() {
                    for target in [&mut transition.unordered_condition, &mut transition.ordered_condition] {
                        *target = Some(target.take().map(|x| format_compact!("{condition} & {x}")).unwrap_or_else(|| condition.clone()));
                    }
                }
                for transition in otherwise_transitions.iter_mut() {
                    transition.unordered_condition = Some(transition.unordered_condition.take().map(|x| format_compact!("~({condition}) & {x}")).unwrap_or_else(|| format_compact!("~({condition})")));
                }

                then_transitions.extend(otherwise_transitions);
                then_transitions
            }
            _ => return Err(CompileError::ComplexTransitionName { state_machine: state_machine.into(), state: state.into() }),
        })
    }

    Ok(match &stmt.kind {
        ast::StmtKind::UnknownBlock { name, args } => match (name.as_str(), args.as_slice()) {
            ("smTransition", [var, value]) => match &var.kind {
                ast::ExprKind::Value(ast::Value::String(var)) if *var == state_machine => Some((parse_transition_target(state_machine, state, value, context)?, None, true)),
                _ => None,
            }
            _ => None,
        }
        ast::StmtKind::Assign { var, value } if var.name == state_machine => match terminal {
            true => Some((parse_transition_target(state_machine, state, value, context)?, None, true)),
            false => return Err(CompileError::NonTerminalTransition { state_machine: state_machine.into(), state: state.into() }),
        }
        ast::StmtKind::If { condition, then } => {
            let condition = translate_expr(state_machine, state, condition, context)?;
            let (mut transitions, _) = parse_stmts(state_machine, state, &then, terminal, context)?;

            for transition in transitions.iter_mut() {
                for target in [&mut transition.unordered_condition, &mut transition.ordered_condition] {
                    *target = Some(target.take().map(|x| format_compact!("{condition} & {x}")).unwrap_or_else(|| condition.clone()));
                }
            }

            Some((transitions, Some(format_compact!("~({condition})")), terminal))
        }
        ast::StmtKind::IfElse { condition, then, otherwise } => {
            let condition = translate_expr(state_machine, state, condition, context)?;

            let (mut transitions_1, body_terminal_1) = parse_stmts(state_machine, state, &then, terminal, context)?;
            let (mut transitions_2, body_terminal_2) = parse_stmts(state_machine, state, &otherwise, terminal, context)?;

            for transition in transitions_1.iter_mut() {
                for target in [&mut transition.unordered_condition, &mut transition.ordered_condition] {
                    *target = Some(target.take().map(|x| format_compact!("{condition} & {x}")).unwrap_or_else(|| condition.clone()));
                }
            }
            for transition in transitions_2.iter_mut() {
                let targets = match body_terminal_1 {
                    true => vec![&mut transition.unordered_condition],
                    false => vec![&mut transition.unordered_condition, &mut transition.ordered_condition],
                };
                for target in targets {
                    *target = Some(target.take().map(|x| format_compact!("~({condition}) & {x}")).unwrap_or_else(|| format_compact!("~({condition})")));
                }
            }

            let tail_condition = match (body_terminal_1, body_terminal_2) {
                (true, true) => Some("false".into()),
                (true, false) => Some(format_compact!("~({condition})")),
                (false, true) => Some(condition.clone()),
                (false, false) => None,
            };

            transitions_1.extend(transitions_2.into_iter());
            Some((transitions_1, tail_condition, terminal || (body_terminal_1 && body_terminal_2)))
        }
        _ => None,
    })
}
fn parse_stmts(state_machine: &str, state: &str, stmts: &[ast::Stmt], mut terminal: bool, context: &mut Context) -> Result<(VecDeque<Transition>, bool), CompileError> {
    let mut actions: VecDeque<CompactString> = <_>::default();
    let mut transitions: VecDeque<Transition> = <_>::default();

    let mut stmts = stmts.iter().rev().peekable();
    while let Some(stmt) = stmts.peek() {
        match &stmt.kind {
            ast::StmtKind::Return { value: _ } => (),
            _ => break,
        }
        stmts.next();
        terminal = true;
    }

    fn handle_actions(state_machine: &str, state: &str, actions: &mut VecDeque<CompactString>, transitions: &mut VecDeque<Transition>, terminal: bool) -> Result<(), CompileError> {
        if !actions.is_empty() {
            if terminal && transitions.is_empty() {
                transitions.push_back(Transition { ordered_condition: None, unordered_condition: None, actions: core::mem::take(actions), new_state: state.into() });
            } else if transitions.len() == 1 && transitions[0].unordered_condition.is_none() {
                transitions[0].actions.extend_front(core::mem::take(actions).into_iter());
            } else {
                return Err(CompileError::ActionsOutsideTransition { state_machine: state_machine.into(), state: state.into() });
            }
        }
        Ok(())
    }

    let mut last = true;
    for stmt in stmts {
        match parse_transitions(state_machine, state, stmt, terminal && last, context)? {
            Some((sub_transitions, tail_condition, sub_terminal)) => {
                handle_actions(state_machine, state, &mut actions, &mut transitions, terminal)?;
                debug_assert_eq!(actions.len(), 0);

                terminal |= sub_terminal;
                if let Some(tail_condition) = tail_condition {
                    for transition in transitions.iter_mut() {
                        transition.unordered_condition = Some(transition.unordered_condition.take().map(|x| format_compact!("{tail_condition} & {x}")).unwrap_or_else(|| tail_condition.clone()));
                    }
                }
                transitions.extend_front(sub_transitions.into_iter());
            }
            None => {
                actions.extend_front(parse_actions(state_machine, state, stmt, context)?.into_iter());
            }
        }
        last = false;
    }

    handle_actions(state_machine, state, &mut actions, &mut transitions, terminal)?;
    debug_assert_eq!(actions.len(), 0);

    Ok((transitions, terminal))
}

fn dot_id(name: &str) -> dot::Id {
    dot::Id::Escaped(format!("{name:?}"))
}

impl Project {
    pub fn compile(xml: &str, role: Option<&str>, settings: Settings) -> Result<Project, CompileError> {
        let parser = ast::Parser {
            name_transformer: Box::new(ast::util::c_ident),
            ..Default::default()
        };
        let proj = parser.parse(xml).map_err(|e| CompileError::ParseError(e))?;
        let role = match role {
            Some(name) => match proj.roles.iter().find(|r| r.name == name) {
                Some(x) => x,
                None => return Err(CompileError::UnknownRole { name: name.into() }),
            }
            None => match proj.roles.as_slice() {
                [x] => x,
                x => return Err(CompileError::RoleCount { count: x.len() }),
            }
        };

        let mut state_machines: BTreeMap<CompactString, StateMachine> = <_>::default();
        let mut visited_handlers: BTreeSet<(CompactString, CompactString)> = <_>::default();

        for entity in role.entities.iter() {
            for script in entity.scripts.iter() {
                let (state_machine_name, state_name) = match script.hat.as_ref().map(|x| &x.kind) {
                    Some(ast::HatKind::When { condition }) => match &condition.kind {
                        ast::ExprKind::Eq { left, right } => match (&left.kind, &right.kind) {
                            (ast::ExprKind::Variable { var }, ast::ExprKind::Value(ast::Value::String(val))) => (&var.name, val),
                            (ast::ExprKind::Value(ast::Value::String(val)), ast::ExprKind::Variable { var }) => (&var.name, val),
                            _ => continue,
                        }
                        ast::ExprKind::UnknownBlock { name, args } => match (name.as_str(), args.as_slice()) {
                            ("smInState", [var, val]) => match (&var.kind, &val.kind) {
                                (ast::ExprKind::Value(ast::Value::String(var)), ast::ExprKind::Value(ast::Value::String(val))) => (var, val),
                                _ => continue,
                            }
                            _ => continue,
                        }
                        _ => continue,
                    }
                    _ => continue,
                };

                if !visited_handlers.insert((state_machine_name.clone(), state_name.clone())) {
                    return Err(CompileError::MultipleHandlers { state_machine: state_machine_name.clone(), state: state_name.clone() });
                }

                let state_machine = state_machines.entry(state_machine_name.clone()).or_insert_with(|| StateMachine { variables: <_>::default(), states: <_>::default(), initial_state: None, current_state: None });
                let state = state_machine.states.entry(state_name.clone()).or_insert_with(|| State { transitions: <_>::default() });
                debug_assert_eq!(state.transitions.len(), 0);

                let mut context = Context { variables: vec![], settings };
                let (transitions, _) = parse_stmts(&state_machine_name, &state_name, &script.stmts, true, &mut context)?;
                let target_states = transitions.iter().map(|x| x.new_state.clone()).collect::<Vec<_>>();
                state.transitions.extend_front(transitions.into_iter());
                for target_state in target_states {
                    state_machine.states.entry(target_state).or_insert_with(|| State { transitions: <_>::default() });
                }
                for variable in context.variables {
                    state_machine.variables.insert(variable.trans_name);
                }
            }
        }

        for entity in role.entities.iter() {
            for script in entity.scripts.iter() {
                if let Some(ast::HatKind::OnFlag) = script.hat.as_ref().map(|x| &x.kind) {
                    for stmt in script.stmts.iter() {
                        let (state_machine_name, initial_state) = match &stmt.kind {
                            ast::StmtKind::Assign { var, value } => match &value.kind {
                                ast::ExprKind::Value(ast::Value::String(value)) => (&var.name, value),
                                _ => continue,
                            }
                            ast::StmtKind::UnknownBlock { name, args } => match (name.as_str(), args.as_slice()) {
                                ("smTransition", [var, value]) => match (&var.kind, &value.kind) {
                                    (ast::ExprKind::Value(ast::Value::String(var)), ast::ExprKind::Value(ast::Value::String(value))) => (var, value),
                                    _ => continue,
                                }
                                _ => continue,
                            }
                            _ => continue,
                        };
                        if let Some(state_machine) = state_machines.get_mut(state_machine_name) {
                            if state_machine.states.contains_key(initial_state) {
                                state_machine.initial_state = Some(initial_state.clone());
                            }
                        }
                    }
                }
            }
        }

        for (state_machine_name, state_machine) in state_machines.iter_mut() {
            if let Some(ast::Value::String(init)) = role.globals.iter().find(|g| g.def.name == state_machine_name).map(|g| &g.init) {
                if state_machine.states.contains_key(init) {
                    state_machine.current_state = Some(init.clone());
                }
            }
        }

        let mut machines = state_machines.iter();
        while let Some(machine_1) = machines.next() {
            if let Some((machine_2, var)) = machines.clone().find_map(|machine_2| machine_1.1.variables.intersection(&machine_2.1.variables).next().map(|x| (machine_2, x))) {
                return Err(CompileError::VariableOverlap { state_machines: (machine_1.0.clone(), machine_2.0.clone()), variable: var.clone() });
            }
            if let Some(var) = machine_1.1.variables.iter().find(|&x| state_machines.contains_key(x)) {
                return Err(CompileError::VariableOverlap { state_machines: (machine_1.0.clone(), var.clone()), variable: var.clone() });
            }
        }

        Ok(Project { name: proj.name, role: role.name.clone(), state_machines })
    }
    pub fn to_graphviz(&self) -> dot::Graph {
        let stmts = self.state_machines.iter().map(|x| dot::Stmt::Subgraph(StateMachine::to_graphviz(x.1, x.0))).collect();
        dot::Graph::DiGraph { id: dot_id(&self.name), strict: false, stmts }
    }
}
impl StateMachine {
    pub fn to_graphviz(&self, name: &str) -> dot::Subgraph {
        let node_id = |state: &str| dot::NodeId(if !state.is_empty() { dot_id(&format!("{name} {state}")) } else { dot_id(name) }, None);

        let mut stmts = vec![];
        if let Some(init) = self.initial_state.as_ref() {
            stmts.push(dot::Stmt::Node(dot::Node { id: node_id(""), attributes: vec![dot::Attribute(dot::Id::Plain("shape".into()), dot::Id::Plain("point".into()))] }));
            stmts.push(dot::Stmt::Edge(dot::Edge { ty: dot::EdgeTy::Pair(dot::Vertex::N(node_id("")), dot::Vertex::N(node_id(init))), attributes: vec![] }));
        }
        for state_name in self.states.keys() {
            let mut attributes = vec![dot::Attribute(dot::Id::Plain("label".into()), dot_id(state_name))];
            if self.current_state.as_ref().map(|x| x == state_name).unwrap_or(false) {
                attributes.push(dot::Attribute(dot::Id::Plain("style".into()), dot::Id::Plain("filled".into())));
            }
            stmts.push(dot::Stmt::Node(dot::Node { id: node_id(state_name), attributes }));
        }
        for (state_name, state) in self.states.iter() {
            let labeler: fn (usize, Option<&str>) -> dot::Id = match state.transitions.len() {
                1 => |_, t| t.map(|t| dot_id(&format!(" {t} "))).unwrap_or_else(|| dot_id("")),
                _ => |i, t| t.map(|t| dot_id(&format!(" {}: {t} ", i + 1))).unwrap_or_else(|| dot_id(&format!(" {} ", i + 1))),
            };
            for (i, transition) in state.transitions.iter().enumerate() {
                stmts.push(dot::Stmt::Edge(dot::Edge { ty: dot::EdgeTy::Pair(dot::Vertex::N(node_id(state_name)), dot::Vertex::N(node_id(&transition.new_state))), attributes: vec![
                    dot::Attribute(dot::Id::Plain("label".into()), labeler(i, transition.ordered_condition.as_deref())),
                ] }));
            }
        }
        dot::Subgraph { id: dot_id(&name), stmts }
    }
}
