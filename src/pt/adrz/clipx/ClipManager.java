package pt.adrz.clipx;
import java.awt.AWTException;
import java.awt.HeadlessException;
import java.awt.MenuItem;
import java.awt.PopupMenu;
import java.awt.SystemTray;
import java.awt.Toolkit;
import java.awt.TrayIcon;
import java.awt.TrayIcon.MessageType;
import java.awt.datatransfer.Clipboard;
import java.awt.datatransfer.ClipboardOwner;
import java.awt.datatransfer.DataFlavor;
import java.awt.datatransfer.FlavorEvent;
import java.awt.datatransfer.FlavorListener;
import java.awt.datatransfer.StringSelection;
import java.awt.datatransfer.Transferable;
import java.awt.datatransfer.UnsupportedFlavorException;
import java.awt.event.ActionEvent;
import java.awt.event.ActionListener;
import java.awt.event.KeyEvent;
import java.awt.event.KeyListener;
import java.awt.event.MouseEvent;
import java.awt.event.MouseListener;
import java.io.IOException;
import java.util.Iterator;
import java.util.LinkedList;

import javax.swing.ImageIcon;
import javax.swing.JFrame;

/**
 * Class that will caught clipboard events, if it is a String, that 
 * String will be stored in a LinkedList
 * @author adriano
 *
 */
public class ClipManager implements FlavorListener, ClipboardOwner, ActionListener, MouseListener, KeyListener{

	private static final String MENU_ITEM_ABOUT 	= "About";
	private static final String MENU_ITEM_EXIT 		= "Exit";
	private static final String TOOL_TIP 			= "ClipX";
	
	/**
	 * SyS clipboard
	 */
	private Clipboard clip;
	
	
	/**
	 * GUI of the application
	 */
	private ClipGUI gui;
	
	// SysTray vars
	private PopupMenu	popupMenu;
	private TrayIcon 	trayIcon;
	private SystemTray 	sysTray;
	
	// SysTray menu items 
	MenuItem item01;
	MenuItem item02;
	MenuItem item03;
	MenuItem mItemAbout;
	MenuItem mItemExit;
	
	
	
	
	
	/**
	 * Constructor - Get and then Set the contents in clipboard in order
	 * to get the ownership.
	 */
	public ClipManager() {
		
		this.clip = Toolkit.getDefaultToolkit().getSystemClipboard();
		
		this.sysTray = SystemTray.getSystemTray();
		
		clip.setContents(clip.getContents(null), this);
		clip.addFlavorListener(this);
		
		gui = new ClipGUI(this);
		
		this.iniSysTray();
		
		this.gui.addKeyListener(this);
	}

	
	
	
	/**
	 * initializated system tray icon resources
	 */
	private void iniSysTray() {
		
		item01 		= new MenuItem("item01");
		item02 		= new MenuItem("item02");
		item03 		= new MenuItem("item03");
		mItemAbout 	= new MenuItem(MENU_ITEM_ABOUT);
		
		mItemAbout.setEnabled(false);
		
		mItemExit	= new MenuItem(MENU_ITEM_EXIT);
		
		item01.addActionListener(this);
		item02.addActionListener(this);
		item03.addActionListener(this);
		mItemAbout.addActionListener(this);
		mItemExit.addActionListener(this);
		
		popupMenu = new PopupMenu();
		popupMenu.add(item01);
		popupMenu.add(item02);
		popupMenu.add(item03);
		popupMenu.add(mItemAbout);
		popupMenu.add(mItemExit);
		popupMenu.addActionListener(this);		
			
		trayIcon = new TrayIcon(new ImageIcon("img/mainIcon.gif", "ClipX").getImage());
		trayIcon.setPopupMenu(popupMenu);
		trayIcon.setImageAutoSize(true);
		trayIcon.addActionListener(this);
		trayIcon.addMouseListener(this);
		
		trayIcon.setToolTip(TOOL_TIP);
		
		try { sysTray.add(trayIcon); }
		catch (AWTException eAWT) { }
		
		trayIcon.displayMessage("ClipX", "clipboard Strings will be Saved ...", MessageType.INFO);
	}
	
	
	
	
	/**
	 * Clipboard has new data
	 */
	@Override
	public void flavorsChanged(FlavorEvent e) {
		
		//clip.removeFlavorListener(this);
		
		Transferable tf = clip.getContents(null);
		
		String copyString = null;
		
		if (tf.isDataFlavorSupported(DataFlavor.stringFlavor)) {		
			try {
				copyString = (String)tf.getTransferData(DataFlavor.stringFlavor);
				if (copyString != null) {
					StringSelection ss = new StringSelection(copyString);
					Toolkit.getDefaultToolkit().getSystemClipboard().setContents(ss, this);
				}
				if(!this.gui.getList().getModel().getItems().contains(copyString)) {
					try {
						gui.getList().getModel().addElementTo(copyString, 0);
						this.gui.getEditTA().setText(copyString);
					}
					catch (Exception ex) {
						ex.printStackTrace();
					}
				}
				else {}
			}
			catch (IOException ioEx) { }
			catch (UnsupportedFlavorException unFlvEx) { }
		}
		else { }
		
		//clip.addFlavorListener(this);
		
		//try { Thread.sleep(1000); } 
		//catch (InterruptedException ex) { ex.printStackTrace(); }
	}
	
	private void flavorChangedFunction() {
		
		// We don't need to be notified when we are copying data from clipboard
		clip.removeFlavorListener(this);
		
		// process event
		String strClip = null;
		Transferable tf = null;
		try {
			tf = clip.getContents(null);
			
			// Check if it's a String
			if (tf.isDataFlavorSupported(DataFlavor.stringFlavor)) {		
				try {
					// get the string
					strClip = (String)tf.getTransferData(DataFlavor.stringFlavor);
					System.out.println("Data from clipboard = " + strClip);
										
					// If this String is not stored, save it...
					if(!this.gui.getList().getModel().getItems().contains(strClip)) {
						try {
							gui.getList().getModel().addElementTo(strClip, 0);
							this.setClipboard(strClip);
							this.gui.getEditTA().setText(strClip);
						}
						catch (Exception ex) {
							ex.printStackTrace();
						}
					}
					else {
						System.out.println("that Clipboard String already exists");
						// next ... the item with the string must be placed in the index 0
						this.setClipboard(strClip);
					}
				}
				catch (IOException eIO) {
					System.out.println("IO error!");
				}
				catch (UnsupportedFlavorException eUFE) {
					System.out.println("flavor not supported");
				}
			}
			else {
				System.out.println("not a String");
				gui.getEditTA().setText("<<< Clipboard Dont have a String item >>>");
			}
		}
		catch (IllegalStateException eISE) {
			System.out.println("cannot get contents from clipboard");
		}
		
		// Get the SyS clipboard and then be his owner by setting the same data
		try {
			clip = Toolkit.getDefaultToolkit().getSystemClipboard();
			clip.setContents(clip.getContents(null), this);
		}
		catch (IllegalStateException eISE) {
			System.out.println("cannot set contents in clipboard");
		}
		catch (HeadlessException eHE) {
			System.out.println("Cannot get clipboard");
		}
		
		// start listening clipboard changes
		clip.addFlavorListener(this);
		
	}
	
	/**
	 * Replace the clipboard with the given string
	 * @param text String to be stored in the clipboard
	 */
	public void setClipboard(String text) {
		// we don't need to be notified about this change
		clip.removeFlavorListener(this);
		
		// add text to clipboard
		StringSelection ss = new StringSelection(text);
		Toolkit.getDefaultToolkit().getSystemClipboard().setContents(ss, this);
		
		// ... from now one we need clipboard changes notifications
		clip.addFlavorListener(this);	
		
		try {
			Thread.sleep(100000L);
		} catch (InterruptedException e) {
			// TODO Auto-generated catch block
			e.printStackTrace();
		} 
	}
	
	
	
	
	
	
	
	
	
	@Override
	public void lostOwnership(Clipboard arg0, Transferable arg1) {
	}



	/**
	 * Every event from systray menu is processed here
	 */
	@Override
	public void actionPerformed(ActionEvent e) {
		try {
			System.out.println(e.getActionCommand());
			if (e.getActionCommand().equals(MENU_ITEM_EXIT)) {	// EXIT
				System.exit(0);
			}	
		}
		catch (NullPointerException eNULL) {
			System.out.println("event is null in Action Performed");
		}
		catch (Exception eEX) {
			System.out.println("unknow exception in Action Performed");
		}	
	}



	/**
	 * 
	 * Process the event when a single left mouse click is done over systray ClipX icon
	 */
	@Override
	public void mouseClicked(MouseEvent e) {
		if (e.getButton() == MouseEvent.BUTTON1) {
			gui.setVisible(true);
		}
	}



	
	
	
	
	
	
	
	
	
	

	@Override
	public void mouseEntered(MouseEvent e) {
		
		
	}




	@Override
	public void mouseExited(MouseEvent e) {
		
		
	}




	@Override
	public void mousePressed(MouseEvent e) {
		
	}




	@Override
	public void mouseReleased(MouseEvent arg0) {
		
		
	}




	@Override
	public void keyPressed(KeyEvent arg0) {
		// TODO Auto-generated method stub
		System.out.println("keypressed = "+ arg0.getKeyCode());
	}




	@Override
	public void keyReleased(KeyEvent arg0) {
		// TODO Auto-generated method stub
		
	}




	@Override
	public void keyTyped(KeyEvent arg0) {
		// TODO Auto-generated method stub
		
	}




	



	

	
}
