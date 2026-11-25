use web_sys::Element;
use yew::prelude::*;

const PAN_INSET: i32 = 128;
const DRAG_THRESHOLD: i32 = 5;

struct Drag {
    pointer: i32,
    x: i32,
    y: i32,
    active: bool,
}

pub fn center_view(viewport: &NodeRef) {
    if let Some(element) = viewport.cast::<Element>() {
        element.set_scroll_left((element.scroll_width() - element.client_width()) / 2);
        element.set_scroll_top(PAN_INSET);
    }
}

#[derive(Properties, PartialEq)]
pub struct GraphViewportProps {
    pub viewport: NodeRef,
    pub children: Children,
    #[prop_or(AttrValue::Static("Scrollable recipe graph"))]
    pub label: AttrValue,
    #[prop_or(AttrValue::Static("graph-instructions"))]
    pub instructions: AttrValue,
    #[prop_or(true)]
    pub center_on_mount: bool,
    #[prop_or_default]
    pub class: Classes,
    #[prop_or_default]
    pub onscroll: Callback<Event>,
}

#[function_component]
pub fn GraphViewport(props: &GraphViewportProps) -> Html {
    let drag = use_mut_ref(|| None::<Drag>);

    {
        let viewport = props.viewport.clone();
        use_effect_with(props.center_on_mount, move |center| {
            if *center {
                center_view(&viewport);
            }
        });
    }

    let onpointerdown = {
        let drag = drag.clone();
        Callback::from(move |event: PointerEvent| {
            if event.is_primary() && event.button() == 0 && event.pointer_type() != "touch" {
                *drag.borrow_mut() = Some(Drag {
                    pointer: event.pointer_id(),
                    x: event.client_x(),
                    y: event.client_y(),
                    active: false,
                });
            }
        })
    };

    let onpointermove = {
        let drag = drag.clone();
        let viewport = props.viewport.clone();
        Callback::from(move |event: PointerEvent| {
            let mut drag = drag.borrow_mut();
            let Some(current) = drag
                .as_mut()
                .filter(|drag| drag.pointer == event.pointer_id())
            else {
                return;
            };
            if event.buttons() & 1 == 0 {
                if current.active
                    && let Some(element) = viewport.cast::<Element>()
                {
                    let _ = element.remove_attribute("data-panning");
                }
                *drag = None;
                return;
            }

            let x = event.client_x();
            let y = event.client_y();
            let dx = x - current.x;
            let dy = y - current.y;
            if !current.active && dx.abs().max(dy.abs()) < DRAG_THRESHOLD {
                return;
            }
            let Some(element) = viewport.cast::<Element>() else {
                return;
            };
            if !current.active {
                // A captured release sends its click to the viewport, leaving the card closed.
                if element.set_pointer_capture(current.pointer).is_err() {
                    return;
                }
                current.active = true;
                let _ = element.set_attribute("data-panning", "");
            }

            element.set_scroll_left(element.scroll_left() - dx);
            element.set_scroll_top(element.scroll_top() - dy);
            current.x = x;
            current.y = y;
        })
    };

    let onpointerleave = {
        let drag = drag.clone();
        Callback::from(move |event: PointerEvent| {
            let mut drag = drag.borrow_mut();
            if drag
                .as_ref()
                .is_some_and(|drag| drag.pointer == event.pointer_id() && !drag.active)
            {
                *drag = None;
            }
        })
    };

    let finish = {
        let viewport = props.viewport.clone();
        Callback::from(move |event: PointerEvent| {
            let mut drag = drag.borrow_mut();
            if drag
                .as_ref()
                .is_some_and(|drag| drag.pointer == event.pointer_id())
            {
                *drag = None;
                if let Some(element) = viewport.cast::<Element>() {
                    let _ = element.remove_attribute("data-panning");
                }
            }
        })
    };

    html! {
        <div class={classes!("graph-viewport", props.class.clone())} tabindex="0" role="region"
            aria-label={props.label.clone()} aria-describedby={props.instructions.clone()}
            ref={props.viewport.clone()} style={format!("--pan-inset: {PAN_INSET}px")}
            onscroll={props.onscroll.clone()}
            {onpointerdown} {onpointermove} {onpointerleave} onpointerup={finish.clone()}
            onpointercancel={finish.clone()} onlostpointercapture={finish}
            ondragstart={Callback::from(|event: DragEvent| event.prevent_default())}>
            {props.children.clone()}
        </div>
    }
}
