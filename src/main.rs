use ash::version::DeviceV1_0;
use ash::version::EntryV1_0;
use ash::version::InstanceV1_0;
use ash::vk;
use glfw::ffi::glfwTerminate;
use raw_window_handle::RawDisplayHandle;
use raw_window_handle::RawWindowHandle;
use raw_window_handle::WaylandDisplayHandle;
use raw_window_handle::WaylandWindowHandle;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
struct DebugThing {
    loader: ash::extensions::ext::DebugUtils,
    messenger: vk::DebugUtilsMessengerEXT
}

impl DebugThing {
     fn init(entry: &ash::Entry, instance: &ash::Instance) -> Result<DebugThing, vk::Result> {
        let mut debugcreateinfo = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                    | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                    | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
            )
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION,
            )
            .pfn_user_callback(Some(vulkan_debug_utils_callback));

        let loader = ash::extensions::ext::DebugUtils::new(entry, instance);
        let messenger = unsafe { loader.create_debug_utils_messenger(&debugcreateinfo, None)? };

        Ok(DebugThing { loader, messenger })
    }
}
impl Drop for DebugThing {
    fn drop(&mut self){
        unsafe{
            self.loader
                .destroy_debug_utils_messenger(self.messenger, None);
        }
    }
}

struct SurfaceThing {
    wayland_surface_loader: ash::extensions::khr::WaylandSurface,
    surface: vk::SurfaceKHR,
    surface_loader: ash::extensions::khr::Surface,
}

impl SurfaceThing {

    fn init(window: &glfw::PWindow, entry: &ash::Entry, instance:&ash::Instance) -> Result<SurfaceThing, vk::Result> {
        let display = window.display_handle().unwrap().as_raw();
        let handle = window.window_handle().unwrap().as_raw();
        let (display, surface) = match (display, handle){
            (RawDisplayHandle::Wayland(WaylandDisplayHandle { display, .. }), RawWindowHandle::Wayland(WaylandWindowHandle { surface,.. })) => (display.as_ptr(), surface.as_ptr()),
            _ => panic!("Not running on wayland")
        };
        let wl_create_info = vk::WaylandSurfaceCreateInfoKHR::builder()
            .display(display)
            .surface(surface)
            .build();
        let wl_surface_loader = ash::extensions::khr::WaylandSurface::new(entry, instance);
        let surface = unsafe { wl_surface_loader.create_wayland_surface(&wl_create_info, None)}?;
        let surface_loader = ash::extensions::khr::Surface::new(entry, instance);
        Ok(SurfaceThing {wayland_surface_loader: wl_surface_loader,surface: surface,surface_loader: surface_loader})
    }

    fn get_capabilities(&self, physical_device: vk::PhysicalDevice) -> Result<vk::SurfaceCapabilitiesKHR, vk::Result> {
        unsafe {
            self.surface_loader.get_physical_device_surface_capabilities(physical_device, self.surface)
        }
    }
    fn get_present_modes(&self, physical_device: vk::PhysicalDevice) -> Result<Vec<vk::PresentModeKHR>, vk::Result>{
        unsafe {
            self.surface_loader.get_physical_device_surface_present_modes(physical_device, self.surface)
        }
    }
    fn get_formats(&self, physical_device: vk::PhysicalDevice) -> Result<Vec<vk::SurfaceFormatKHR>, vk::Result> {
        unsafe {
            self.surface_loader.get_physical_device_surface_formats(physical_device, self.surface)
        }
    }
    fn get_physical_device_surface_support(&self, physical_device: vk::PhysicalDevice, queuefamilyindex: usize) -> Result<bool, vk::Result>{
        unsafe {
            self.surface_loader.get_physical_device_surface_support(physical_device, queuefamilyindex as u32, self.surface)
        }
    }
}
impl Drop for SurfaceThing{
    fn drop(&mut self) {
        unsafe {
            self.surface_loader.destroy_surface(self.surface, None);
        }
    }
}

struct QueueFamilies {
    graphics_q_index : Option<u32>,
    transfer_q_index : Option<u32>,
}
impl QueueFamilies {
    fn init(instance: &ash::Instance, physical_device: vk::PhysicalDevice, surface: &SurfaceThing) -> Result<QueueFamilies, vk::Result> {
        let queuefamilyproperties = unsafe { instance.get_physical_device_queue_family_properties(physical_device)};
        let mut found_graphics_q_index = None;
        let mut found_transfer_q_index = None;

        for (index, qfam) in queuefamilyproperties.iter().enumerate(){
            if qfam.queue_count > 0 && qfam.queue_flags.contains(vk::QueueFlags::GRAPHICS) && surface.get_physical_device_surface_support(physical_device, index)? {
                found_graphics_q_index = Some(index as u32)
            }
            if qfam.queue_count > 0 && qfam.queue_flags.contains(vk::QueueFlags::TRANSFER){
                if found_transfer_q_index.is_none() || !qfam.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                    found_transfer_q_index = Some(index as u32)
                }
            }
        }
        Ok(QueueFamilies {
            graphics_q_index: found_graphics_q_index,
            transfer_q_index: found_transfer_q_index
        })
    }
   
}

struct Queues {
    graphics_queue: vk::Queue,
    transfer_queue: vk::Queue
}

fn init_device_and_queues(instance: &ash::Instance, physical_device: vk::PhysicalDevice,queue_families: &QueueFamilies, layer_names: &[&str]) -> Result<(ash::Device, Queues), vk::Result> {
    let layer_names_c: Vec<std::ffi::CString> = layer_names
        .iter()
        .map(|&ln| std::ffi::CString::new(ln).unwrap())
        .collect();
    let layer_name_pointers: Vec<*const i8> = layer_names_c
        .iter()
        .map(|layer_name| layer_name.as_ptr())
        .collect();
    let priorities = [1.0f32];
    let queue_infos = [
        vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(queue_families.graphics_q_index.unwrap())
            .queue_priorities(&priorities)
            .build(),
        vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(queue_families.transfer_q_index.unwrap())
            .queue_priorities(&priorities)
            .build(),
    ];
    let device_extention_name_pointers: Vec<*const i8> =
        vec![ash::extensions::khr::Swapchain::name().as_ptr()];

    let device_create_info = vk::DeviceCreateInfo::builder()
        .queue_create_infos(&queue_infos)
        .enabled_extension_names(&device_extention_name_pointers)
        .enabled_layer_names(&layer_name_pointers);
    let logical_device = 
        unsafe { instance.create_device(physical_device, &device_create_info, None)?};
    let graphics_queue = unsafe {logical_device.get_device_queue(queue_families.graphics_q_index.unwrap(), 0)};
    let transfer_queue = unsafe { logical_device.get_device_queue(queue_families.transfer_q_index.unwrap(), 0)};
    Ok((logical_device, 
    Queues {
        graphics_queue,
        transfer_queue
    }))
}


fn init_instance( entry: &ash::Entry, layer_names: &Vec<&str>, ) -> Result<ash::Instance, ash::InstanceError> {
    let enginename = std::ffi::CString::new("GubbyEngine").unwrap();
    let appname = std::ffi::CString::new("GubbyGaming213").unwrap();
    let app_info = vk::ApplicationInfo::builder().application_name(&appname)
        .application_version(vk::make_version(0, 0, 1))
        .engine_name(&enginename)
        .engine_version(vk::make_version(0, 42, 0))            .api_version(vk::make_version(1, 0, 106));

   let layer_names_c: Vec<std::ffi::CString> = layer_names
        .iter()
        .map(|&ln| std::ffi::CString::new(ln).unwrap())
        .collect();
    let layer_name_pointers: Vec<*const i8> = layer_names_c
        .iter()
        .map(|layer_name| layer_name.as_ptr())
        .collect();
    let extension_name_pointers : Vec<*const i8> =vec![
        ash::extensions::ext::DebugUtils::name().as_ptr(),
        ash::extensions::khr::Surface::name().as_ptr(),
        ash::extensions::khr::WaylandSurface::name().as_ptr()        
    ];
    let instance_create_info = vk::InstanceCreateInfo::builder()
        .application_info(&app_info)
        .enabled_layer_names(&layer_name_pointers)
        .enabled_extension_names(&extension_name_pointers);

    unsafe { entry.create_instance(&instance_create_info, None) }
    
}

fn init_phys_device_and_properties(instance: &ash::Instance) -> Result<(vk::PhysicalDevice, vk::PhysicalDeviceProperties), vk::Result>{
    let phys_devs = unsafe { instance.enumerate_physical_devices()?};
    let mut chosen = None;
    for p in phys_devs {
        let props = unsafe {
            instance.get_physical_device_properties(p)
        };
        if props.device_type == vk::PhysicalDeviceType::DISCRETE_GPU {
            chosen = Some((p,props));
        }
    }
    Ok(chosen.unwrap())
}
struct SwapchainThing {
    swapchain_loader: ash::extensions::khr::Swapchain,
    swapchain : vk::SwapchainKHR,
    image: Vec<vk::Image>,
    imageviews: Vec<vk::ImageView>,
    framebuffers: Vec<vk::Framebuffer>,
    surface_format: vk::SurfaceFormatKHR,
    extent: vk::Extent2D
}
impl SwapchainThing {
    fn init(instance: &ash::Instance, physical_device: vk::PhysicalDevice, logical_device: &ash::Device, surfaces: &SurfaceThing, queue_families: &QueueFamilies, queues: &Queues)->Result<SwapchainThing, vk::Result> {
        let extents : vk::Extent2D = vk::Extent2D::builder().
        width(WIDTH)
        .height(HEIGHT).build();
        
        let surface_capabilities = surfaces.get_capabilities(physical_device)?;
        let surface_present_modes = surfaces.get_present_modes(physical_device)?;        let queuefamilies = [queue_families.graphics_q_index.unwrap()];
        let surface_format = *surfaces.get_formats(physical_device)?.first().unwrap();
        
        let swapchain_create_info = vk::SwapchainCreateInfoKHR::builder()
            .surface(surfaces.surface)
            .min_image_count(3.max(surface_capabilities.min_image_count))
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .image_extent(extents)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .queue_family_indices(&queuefamilies)
            .pre_transform(surface_capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(vk::PresentModeKHR::FIFO);
        let swapchain_loader = ash::extensions::khr::Swapchain::new(instance, logical_device);
        let swapchain = unsafe { swapchain_loader.create_swapchain(&swapchain_create_info, None)? };
    
        let swapchain_images = unsafe { swapchain_loader.get_swapchain_images(swapchain)? };
        let mut swapchain_imageviews = Vec::with_capacity(swapchain_images.len());
        for img in &swapchain_images {
            let subresource_range = vk::ImageSubresourceRange::builder()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .base_mip_level(0)
                .level_count(1)
                .base_array_layer(0)
                .layer_count(1);
            let imageview_create_info = vk::ImageViewCreateInfo::builder()
                .image(*img)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(vk::Format::R16G16B16A16_SFLOAT)
                .subresource_range(*subresource_range);
            let imageview = unsafe { logical_device.create_image_view(&imageview_create_info, None)?};
            swapchain_imageviews.push(imageview);
        }
        Ok(SwapchainThing { swapchain_loader, swapchain, image: swapchain_images, imageviews: swapchain_imageviews, framebuffers: vec![], surface_format, extent: extents })
    }

    fn create_framebuffers(&mut self, logical_device: &ash::Device, renderpass: vk::RenderPass) -> Result<(), vk::Result>{
        for iv in &self.imageviews {
            let iview = [*iv];
            let framebuffer_info = vk::FramebufferCreateInfo::builder()
                .render_pass(renderpass)
                .attachments(&iview)
                .width(self.extent.width)
                .height(self.extent.height)
                .layers(1);
            let fb = unsafe {logical_device.create_framebuffer(&framebuffer_info, None)}?;
            self.framebuffers.push(fb);
        }
        Ok(())
    }

    unsafe fn cleanup(&mut self, logical_device: &ash::Device){
        for fb in &self.framebuffers {
            logical_device.destroy_framebuffer(*fb, None);
        }
        for iv in &self.imageviews {
           logical_device.destroy_image_view(*iv, None);
        }
        self.swapchain_loader.destroy_swapchain(self.swapchain, None);
    }
}

fn init_renderpass(logical_device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    surfaces: &SurfaceThing,
    ) -> Result<vk::RenderPass, vk::Result> {
        let attachments = [vk::AttachmentDescription::builder()
            .format(surfaces.
                get_formats(physical_device)?
                .first()
                .unwrap()
                .format,
                )
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .samples(vk::SampleCountFlags::TYPE_1)
            .build()];

        let color_attachement_references = [vk::AttachmentReference {
            attachment:0,
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        }];
        let subpasses = [vk::SubpassDescription::builder()
            .color_attachments(&color_attachement_references)
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS).build()];
        let subpass_dependencies = [vk::SubpassDependency::builder()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_subpass(0)
            .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
            .build()];
        let renderpass_info = vk::RenderPassCreateInfo::builder()
            .attachments(&attachments)
            .subpasses(&subpasses)
            .dependencies(&subpass_dependencies);
        let renderpass = unsafe { logical_device.create_render_pass(&renderpass_info, None)? };
        Ok(renderpass)
}


struct Renderer {
    window: glfw::PWindow,
    entry: ash::Entry,
    instance: ash::Instance,
    debug:std::mem::ManuallyDrop<DebugThing>,
    surfaces:std::mem::ManuallyDrop<SurfaceThing>,
    physical_device: vk::PhysicalDevice,
    physical_device_properties: vk::PhysicalDeviceProperties,
    queue_families: QueueFamilies,
    queues: Queues,
    device: ash::Device,
    swapchain: SwapchainThing,
    renderpass : vk::RenderPass
}


impl Renderer{
    fn init(window: glfw::PWindow) -> Result<Renderer, Box<dyn std::error::Error>> {
        let entry = ash::Entry::new()?;
        let layer_names = vec!["VK_LAYER_KHRONOS_validation"];
        let instance = init_instance(&entry, &layer_names)?;
        let debug = DebugThing::init(&entry, &instance)?;
        let surfaces = SurfaceThing::init(&window, &entry, &instance)?;

        let (physical_device, physical_device_properties) = 
            init_phys_device_and_properties(&instance)?;
        let queue_families = QueueFamilies::init(&instance, physical_device, &surfaces)?;
        let (logical_device, queues) = 
            init_device_and_queues(&instance, physical_device, &queue_families, &layer_names)?;
        let swapchain = SwapchainThing::init(
            &instance,
            physical_device,
            &logical_device,
            &surfaces,
            &queue_families,
            &queues,
        )?;

        let renderpass = init_renderpass(&logical_device, physical_device, &surfaces)?;
        Ok(Renderer { window, entry, instance, debug : std::mem::ManuallyDrop::new(debug),surfaces : std::mem::ManuallyDrop::new(surfaces), physical_device, physical_device_properties, queue_families, queues, device: logical_device, swapchain, renderpass })
    }

 
}
impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_render_pass(self.renderpass, None);
            self.swapchain.cleanup(&self.device);
            self.device.destroy_device(None);
            std::mem::ManuallyDrop::drop(&mut self.surfaces);
            std::mem::ManuallyDrop::drop(&mut self.debug);
            self.instance.destroy_instance(None);
            
        };
    }
}
const WIDTH : u32 = 600;
const HEIGHT : u32 = 600;




fn main() -> Result<(), Box<dyn std::error::Error>> {
    use glfw::fail_on_errors;
    let mut glfw = glfw::init(fail_on_errors!()).unwrap();
    glfw.window_hint(glfw::WindowHint::ClientApi(glfw::ClientApiHint::NoApi));
    glfw.window_hint(glfw::WindowHint::Resizable(false));
    let (mut window, events) = glfw.create_window(WIDTH, HEIGHT, "sideways stairs", glfw::WindowMode::Windowed).expect("errored window sory");   
    window.set_key_polling(true);
 
    let mut gubbyrenderer = Renderer::init(window).unwrap();
    
    while !gubbyrenderer.window.should_close(){
        glfw.poll_events();
        for(_, event) in  glfw::flush_messages(&events){
            match event {
                glfw::WindowEvent::Key(glfw::Key::Escape, _ ,glfw::Action::Press, _) => {
                    gubbyrenderer.window.set_should_close(true);
                },
               
                _ => {},
            }
        }
            
    }
    unsafe {
        glfwTerminate();
    }
  
    Ok(())
}

unsafe extern "system" fn vulkan_debug_utils_callback(msg_severity: vk::DebugUtilsMessageSeverityFlagsEXT, message_type: vk::DebugUtilsMessageTypeFlagsEXT, p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT, _p_user_data: *mut std::ffi::c_void) -> vk::Bool32 {
    let message = std::ffi::CStr::from_ptr((*p_callback_data).p_message);
    let severity = format!("{:?}", msg_severity).to_lowercase();
    let ty = format!("{:?}", message_type).to_lowercase();
    println!("[Debug[{}][{}] {:?}", severity, ty, message);
    vk::FALSE 
}