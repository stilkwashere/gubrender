
use ash::version::DeviceV1_0;
use ash::version::EntryV1_0;
use ash::version::InstanceV1_0;
use ash::vk;

use winit::raw_window_handle::HasDisplayHandle;
use winit::raw_window_handle::HasWindowHandle;
use winit::raw_window_handle::RawDisplayHandle;
use winit::raw_window_handle::RawWindowHandle;

use winit::window::Window;

struct DebugDongXi {
    loader: ash::extensions::ext::DebugUtils,
    messenger: vk::DebugUtilsMessengerEXT
}

impl DebugDongXi {
     fn init(entry: &ash::Entry, instance: &ash::Instance) -> Result<DebugDongXi, vk::Result> {
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

        Ok(DebugDongXi { loader, messenger })
    }
}
impl Drop for DebugDongXi {
    fn drop(&mut self){
        unsafe{
            self.loader
                .destroy_debug_utils_messenger(self.messenger, None);
        }
    }
}

struct SurfaceDongXi {
    wayland_surface_loader: ash::extensions::khr::WaylandSurface,
    surface: vk::SurfaceKHR,
    surface_loader: ash::extensions::khr::Surface,
}

impl SurfaceDongXi {
    fn init(window: &winit::window::Window, entry: &ash::Entry, instance:&ash::Instance) -> Result<SurfaceDongXi, vk::Result> {
        let display = window.display_handle().unwrap().as_raw();
        let handle = window.window_handle().unwrap().as_raw();
        let (display, surface) = match (display, handle){
            (RawDisplayHandle::Wayland(d), RawWindowHandle::Wayland(w)) => (d.display.as_ptr(), w.surface.as_ptr()),
            _ => panic!("Not running on wayland")
        };
        let wl_create_info = vk::WaylandSurfaceCreateInfoKHR::builder()
            .display(display)
            .surface(surface)
            .build();
        let wl_surface_loader = ash::extensions::khr::WaylandSurface::new(entry, instance);
        let surface = unsafe { wl_surface_loader.create_wayland_surface(&wl_create_info, None)}?;
        let surface_loader = ash::extensions::khr::Surface::new(entry, instance);
        Ok(SurfaceDongXi {wayland_surface_loader: wl_surface_loader,surface: surface,surface_loader: surface_loader})
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
impl Drop for SurfaceDongXi{
    fn drop(&mut self) {
        unsafe {
            self.surface_loader.destroy_surface(self.surface, None);
        }
    }
}
fn init_instance( entry: &ash::Entry, layer_names: Vec<&str>, ) -> Result<ash::Instance, ash::InstanceError> {
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


const WIDTH : u32 = 67;
const HEIGHT : u32 = 68;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let eventloop = winit::event_loop::EventLoop::new().unwrap();
    let window = eventloop.create_window(Window::default_attributes())?;
    let entry = ash::Entry::new()?;
    let instance = init_instance(&entry, vec!["VK_LAYER_KHRONOS_validation"])?;
    let debug_utils = ash::extensions::ext::DebugUtils::new(&entry, &instance);
    let debug = DebugDongXi::init(&entry, &instance)?;
    let phys_devs = unsafe { instance.enumerate_physical_devices()? };

    let (physical_device, physical_device_properties) = {
        let mut chosen = None;
        for p in phys_devs {
            let properties = unsafe { instance.get_physical_device_properties(p)};
            //let name = String::from(
           //     unsafe { std::ffi::CStr::from_ptr(properties.device_name.as_ptr()).to_str().unwrap()}
            //);
           // if name == "AMD Radeon RX 6600 XT (RADV NAVI23)" {
            //    chosen = Some((p, properties));
          //  }
          if properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU {
            chosen = Some((p, properties));
          }
        }
        chosen.unwrap()
    };
    let surfaces = SurfaceDongXi::init(&window, &entry, &instance)?;
    let queuefamilyproperties = unsafe { instance.get_physical_device_queue_family_properties(physical_device) };
    dbg!(&queuefamilyproperties);
    let qfamindices = {
        let mut found_graphics_q_index = None;
        let mut found_transfer_q_index = None;
        for (index, qfam) in queuefamilyproperties.iter().enumerate() {
            if qfam.queue_count > 0 && qfam.queue_flags.contains(vk::QueueFlags::GRAPHICS) && unsafe {surfaces.surface_loader.get_physical_device_surface_support(physical_device, index as u32, surfaces.surface)?}
            {
                found_graphics_q_index = Some(index as u32);
            }
            if qfam.queue_count > 0 && qfam.queue_flags.contains(vk::QueueFlags::TRANSFER){
                found_transfer_q_index = Some(index as u32)
            }
        }
        (found_graphics_q_index.unwrap(), found_transfer_q_index.unwrap())
    };

    let priorities = [1.0f32];
    let queue_infos = [
        vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(qfamindices.0)
            .queue_priorities(&priorities)
            .build(),
        vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(qfamindices.1)
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
    let graphics_queue = unsafe {logical_device.get_device_queue(qfamindices.0, 0)};
    let transfer_queue = unsafe { logical_device.get_device_queue(qfamindices.1, 0)};

   let surface_capabilities = unsafe {
        surfaces.get_capabilities(physical_device)?
   };
   let surface_present_modes = unsafe {
        surfaces.get_present_modes(physical_device)?
    };
   let surface_formats = unsafe {
        surfaces.get_formats(physical_device)?
    };
  
   // dbg!(&surface_capabilities);
   // dbg!(&surface_present_modes);
   //dbg!(&surface_formats);
   let extents = vk::Extent2D {
        width : WIDTH,
        height: HEIGHT
        
    };
   let queuefamilies = [qfamindices.0];
   let swapchain_create_info = vk::SwapchainCreateInfoKHR::builder()
    .surface(surfaces.surface)
    .min_image_count(3.max(surface_capabilities.min_image_count))
    .image_format(surface_formats.first().unwrap().format)
    .image_color_space(surface_formats.first().unwrap().color_space)
    .image_extent(extents)
    .image_array_layers(1)
    .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
    .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
    .queue_family_indices(&queuefamilies)
    .pre_transform(surface_capabilities.current_transform)
    .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
    .present_mode(vk::PresentModeKHR::FIFO);
   let swapchain_loader = ash::extensions::khr::Swapchain::new(&instance, &logical_device);
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
            .format(vk::Format::B8G8R8A8_UNORM)
            .subresource_range(*subresource_range);
        let imageview = unsafe { logical_device.create_image_view(&imageview_create_info, None)?};
        swapchain_imageviews.push(imageview);
   }


    //Cleanup
    unsafe { 
        for iv in &swapchain_imageviews {
            logical_device.destroy_image_view(*iv, None);
        }
        swapchain_loader.destroy_swapchain(swapchain,None);
        std::mem::drop(surfaces);
        logical_device.destroy_device(None);
        std::mem::drop(debug);
        instance.destroy_instance(None) };
    println!("Deleting instance spongeboy me bob");
    Ok(())
}

unsafe extern "system" fn vulkan_debug_utils_callback(msg_severity: vk::DebugUtilsMessageSeverityFlagsEXT, message_type: vk::DebugUtilsMessageTypeFlagsEXT, p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT, _p_user_data: *mut std::ffi::c_void) -> vk::Bool32 {
    let message = std::ffi::CStr::from_ptr((*p_callback_data).p_message);
    let severity = format!("{:?}", msg_severity).to_lowercase();
    let ty = format!("{:?}", message_type).to_lowercase();
    println!("[Debug[{}][{}] {:?}", severity, ty, message);
    vk::FALSE 
}