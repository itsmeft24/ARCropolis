use std::{io::{Seek, SeekFrom, Read}, time::Duration};

use skyline::{hook, nn, println};
use smash_arc::{ArcLookup, FilePathIdx};

use super::*;
use crate::resource::*;


pub fn get_redirected_load_state_recursive(dir: *const LoadedDirectory) -> LoadState {
    unsafe {
        let load_state = (*dir).state;
        if !(*dir).redirection_directory.is_null() {
            let redirected_load_state = get_redirected_load_state_recursive((*dir).redirection_directory);
            if redirected_load_state >= load_state {
                return load_state;
            } else {
                return redirected_load_state;
            }
        }
        return load_state;
    }
}

#[hook(offset = 0x3542660)]
pub unsafe fn res_loading_thread(service: &mut ResServiceNX) {
    let mut data_arc = std::fs::File::open("rom:/data.arc").unwrap();
    println!("[arcropolis::ResLoadingThread] Thread started!");
    if !service.should_terminate {
        loop {
            while !service.should_terminate {
                if service.loading_thread_state != 2 {
                    (*service.res_update_event).wait();
                }
                if service.should_terminate {
                    return
                }

                // Lock the res service while we collect load requests and free/delete the ResLists.
                nn::os::LockMutex(service.mutex);
                let mut load_requests = Vec::new();

                for res_list_idx in 0..5 {
                    for node in service.res_lists[res_list_idx].into_iter() {
                        // Here we replace mass-load requests with individual ones (Because I am lazy)
                        match node.ty {
                            LoadType::LoadFromFilePackage => {
                                // Lock the filesystem
                                nn::os::LockMutex(service.filesystem_info.mutex);
                                let arc = &service.filesystem_info.path_info.arc;
                                for info in arc.get_file_infos()[arc.get_dir_infos()[node.directory_index as usize].file_info_range()].iter() {
                                    load_requests.push(LoadInfo {
                                        ty: LoadType::StandaloneFile,
                                        filepath_index: info.file_path_index.0,
                                        directory_index: 0xFF_FFFF,
                                        files_to_load: 0,
                                    });
                                }
                                load_requests.push(*node);
                                nn::os::UnlockMutex(service.filesystem_info.mutex);
                            },
                            LoadType::StandaloneFile => {
                                load_requests.push(*node);
                            },
                        }
                    }
                }
                for res_list_idx in 0..5 {
                    service.res_lists[res_list_idx].delete();
                }
                // unlock the res service
                nn::os::UnlockMutex(service.mutex);

                for load_request in load_requests {
                    println!("[arcropolis::ResLoadingThread] Tending to load request: {:#?}", load_request);
                    match load_request.ty {
                        LoadType::LoadFromFilePackage => {
                            
                            if load_request.directory_index != 0xFF_FFFF {
                                nn::os::LockMutex(service.filesystem_info.mutex);
                                let loaded_dir = &mut service.filesystem_info.get_loaded_directories()[load_request.directory_index as usize];
                                if loaded_dir.file_group_index != 0xFF_FFFF && get_redirected_load_state_recursive(loaded_dir) == LoadState::Unloaded {
                                    loaded_dir.state = LoadState::Unknown;
                                }
                                nn::os::UnlockMutex(service.filesystem_info.mutex);
                                
                                let arc = &service.filesystem_info.path_info.arc;
                                let dir_info = &arc.get_dir_infos()[load_request.directory_index as usize];

                                if dir_info.flags.is_symlink()

                            }
                            //panic!("Mass-loading is not implemented yet!")
                            
                        },
                        LoadType::StandaloneFile => {
                            let file_path_index = load_request.filepath_index as usize;
                            nn::os::LockMutex(service.filesystem_info.mutex);
                            // Bounds-check the loaded file path table.
                            if file_path_index < service.filesystem_info.get_loaded_filepaths().len() {
                                // Get the loaded file path, and check to see if its being used.
                                let loaded_filepath = &service.filesystem_info.get_loaded_filepaths()[file_path_index];
                                if loaded_filepath.is_loaded > 0 {
                                    // Get the loaded data and ensure that it is also being used. If so, continue on.
                                    // If the LoadedData's data pointer is already populated, we don't want to do anything.
                                    let loaded_data = &service.filesystem_info.get_loaded_datas()[loaded_filepath.loaded_data_index as usize];
                                    if loaded_data.is_used && loaded_data.data == std::ptr::null() {
                                        nn::os::UnlockMutex(service.filesystem_info.mutex);

                                        let arc = &service.filesystem_info.path_info.arc;

                                        // Perform a standard arc file lookup.
                                        let file_path = &arc.get_file_paths()[file_path_index];
                                        let file_info_index = &arc.get_file_info_indices()[file_path.path.index() as usize];
                                        let file_info = &arc.get_file_infos()[file_info_index.file_info_index];
                                        let dir_offset = &arc.get_folder_offsets()[file_info_index.dir_offset_index as usize];
                                        let mut info_to_data_index = file_info.info_to_data_index.0 as usize;

                                        if file_info.flags.is_localized() {
                                            info_to_data_index += service.locale_idx as usize;
                                        }
                                        if file_info.flags.is_regional() {
                                            info_to_data_index += service.language_idx as usize;
                                        }

                                        let file_info_to_data = &arc.get_file_info_to_datas()[info_to_data_index];
                                        let file_data = &arc.get_file_datas()[file_info_to_data.file_data_index];

                                        // Calculate the offset of the file.
                                        let offset =
                                            arc.file_section_offset as usize + dir_offset.offset as usize + (file_data.offset_in_folder as usize)
                                                << 2;

                                        // Set the offset into read to 0.
                                        service.offset_into_read = 0;

                                        if !service.should_terminate {
                                            loop {
                                                println!("[arcropolis::ResLoadingThread] Reading chunk from data.arc. off_into_read: {}", service.offset_into_read);
                                                (*service.semaphore1).acquire();

                                                // Set the seek on the File_NX instance.
                                                data_arc.seek(SeekFrom::Start(offset as u64 + service.offset_into_read as u64));
                                                //(*(*service.data_arc_filenx)).set_position(offset + service.offset_into_read as usize);

                                                // Read a chunk into the res service buffer.
                                                if service.offset_into_read + service.buffer_size >= file_data.comp_size as usize {
                                                    let buffer = std::slice::from_raw_parts_mut(
                                                        service.buffer_array[service.buffer_array_idx as usize],
                                                        file_data.comp_size as usize - service.offset_into_read,
                                                    );
                                                    // (*(*service.data_arc_filenx)).read(buffer);
                                                    data_arc.read_exact(buffer);
                                                    service.offset_into_read = file_data.comp_size as usize;
                                                } else {
                                                    let buffer = std::slice::from_raw_parts_mut(
                                                        service.buffer_array[service.buffer_array_idx as usize],
                                                        service.buffer_size,
                                                    );
                                                    data_arc.read_exact(buffer);
                                                    // (*(*service.data_arc_filenx)).read(buffer);
                                                    service.offset_into_read += service.buffer_size;
                                                }

                                                (*service.semaphore2).acquire();

                                                // Prepare for the io swap event.
                                                service.processing_file_idx_start = file_info_index.file_info_index.0 & 0xffffff;
                                                service.processing_file_idx_curr = 0;
                                                service.processing_file_idx_count = 1;
                                                service.processing_type = LoadingType::StandaloneFile;
                                                service.processing_dir_idx_start = 0xffffff;
                                                service.processing_dir_idx_single = 0xffffff;
                                                service.current_dir_index = 0xffffff;

                                                service.data_ptr = service.buffer_array[service.buffer_array_idx as usize];

                                                // Signal the io swap event. Let ResInflateThread know its time to do its thing.
                                                (*service.io_swap_event).signal();

                                                // Toggle which buffer to use in the next chunk read.
                                                service.buffer_array_idx ^= 1;

                                                if service.offset_into_read >= file_data.comp_size as usize {
                                                    break
                                                }
                                                // All chunks have been read. We're done here!
                                                service.current_index_loaded_status = true;
                                                *std::mem::transmute::<&LoadState, *mut u32>(&loaded_data.state) = 0xffffffff;
                                            }
                                        }
                                    } else {
                                        nn::os::UnlockMutex(service.filesystem_info.mutex);
                                    }
                                } else {
                                    nn::os::UnlockMutex(service.filesystem_info.mutex);
                                }
                            } else {
                                nn::os::UnlockMutex(service.filesystem_info.mutex);
                            }
                        },
                    }
                }
            }
        }
    }
}

#[hook(offset = 0x3543a90)]
pub fn res_inflate_thread(service: &mut ResServiceNX) {}

pub fn install() {
    println!("[arcropolis::threads::install] Installing thread replacements!");
    skyline::install_hooks!(res_loading_thread);
}
