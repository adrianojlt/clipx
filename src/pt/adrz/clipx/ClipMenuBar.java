package pt.adrz.clipx;

import java.awt.MenuBar;
import java.awt.event.ActionEvent;
import java.awt.event.ActionListener;
import java.awt.event.ItemEvent;
import java.awt.event.ItemListener;

import javax.swing.JCheckBoxMenuItem;
import javax.swing.JMenu;
import javax.swing.JMenuBar;
import javax.swing.JMenuItem;

public class ClipMenuBar extends JMenuBar implements ItemListener, ActionListener{
	
	private static final long serialVersionUID = 1L;
	
	private static final String MENU_ITEM_EXIT 			= "Exit";
	private static final String MENU_ITEM_OPEN 			= "Open";
	private static final String MENU_ITEM_SAVE 			= "Save";
	private static final String MENU_ITEM_SAVE_AS 		= "Save As ...";
	private static final String MENU_ITEM_IMPORT 		= "Import <-";
	private static final String MENU_ITEM_EXPORT 		= "Export ->";
	private static final String MENU_ITEM_ACTIVATE 		= "Activate";
	private static final String MENU_ITEM_PREFERENCES 	= "Preferences";

	private JMenu mFile, mEdit, mOptions, mAbout;

	private JMenuItem itemOpen, itemSave, itemSaveAs, itemImpItems, itemExpItems, itemExit, itemPreferences;
	private JCheckBoxMenuItem itemActivate;
	
	private ClipOptions opt;
	
	private EnableListener enableListener;
	
	
	public ClipMenuBar() {
		
		mFile = new JMenu("File");
		mEdit = new JMenu("Edit");
		mOptions = new JMenu("About");
		mAbout = new JMenu("Options");
		
		itemOpen = new JMenuItem(MENU_ITEM_OPEN);
		itemSave = new JMenuItem(MENU_ITEM_SAVE);
		itemSaveAs = new JMenuItem(MENU_ITEM_SAVE_AS);
		itemImpItems = new JMenuItem(MENU_ITEM_IMPORT);
		itemExpItems = new JMenuItem(MENU_ITEM_EXPORT);
		itemExit = new JMenuItem(MENU_ITEM_EXIT);
		itemPreferences = new JMenuItem(MENU_ITEM_PREFERENCES);
		itemActivate = new JCheckBoxMenuItem(MENU_ITEM_ACTIVATE);
		itemActivate.setEnabled(true);
		
		itemActivate.addItemListener(this);
		itemExit.addActionListener(this);
		
		mFile.add(itemOpen);
		mFile.add(itemSave);
		mFile.add(itemSaveAs);
		mFile.add(itemImpItems);
		mFile.add(itemExpItems);
		mFile.add(itemExit);
		mEdit.add(itemPreferences);
		//mOptions.add(itemActivate);
		mOptions.add(itemPreferences);
		
		this.add(mFile);
		this.add(mEdit);
		this.add(mOptions);
		this.add(mAbout);
	}
	
	public void setEnableListener(EnableListener el) {
		this.enableListener = el;
	}
	
	public void setOptions(ClipOptions opt) {
		this.opt = opt;
	}
	
	@Override
	public void itemStateChanged(ItemEvent e) {
		
		if ( e.getItem().equals(MENU_ITEM_ACTIVATE) ) {
			if ( itemActivate.getState() ) {
				this.opt.enable();
				this.enableListener.getClipboardOwnership();
			}
			else {
				this.opt.disable();
			}
		}
	}

	@Override
	public void actionPerformed(ActionEvent e) {

		if ( e.getActionCommand().equals(MENU_ITEM_EXIT) ) {
			System.exit(0);
		}
	}
}
